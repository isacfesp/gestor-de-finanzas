// =====================================================================
// handlers.rs — Pagos e ingresos previstos (flujo de caja proyectado).
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::categorias::validar_categoria;
use crate::accounts::validar_cuenta;
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::planned_transactions::models::{DatosPrevisto, FiltrosPrevistos, Previsto};

fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    if tipo == "income" || tipo == "expense" {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El tipo debe ser 'income' o 'expense'".to_string(),
        ))
    }
}

fn validar_monto(monto: Decimal) -> Result<(), AppError> {
    if monto > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El monto debe ser mayor a cero".to_string(),
        ))
    }
}

/// Valida tipo, monto y, si vienen, que categoría y cuenta sean
/// visibles desde el workspace.
async fn validar_datos(
    pool: &PgPool,
    workspace_id: Uuid,
    datos: &DatosPrevisto,
) -> Result<(), AppError> {
    validar_tipo(&datos.tipo)?;
    validar_monto(datos.amount)?;
    if let Some(category_id) = datos.category_id {
        validar_categoria(pool, category_id, workspace_id, &datos.tipo).await?;
    }
    if let Some(account_id) = datos.account_id {
        validar_cuenta(pool, account_id, workspace_id).await?;
    }
    Ok(())
}

/// POST /workspaces/:workspace_id/previstos
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<DatosPrevisto>,
) -> Result<(StatusCode, Json<Previsto>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_datos(&pool, workspace_id, &datos).await?;

    let fila = sqlx::query_as!(
        Previsto,
        r#"INSERT INTO planned_transactions
               (workspace_id, type, amount, due_date, category_id, account_id, description, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, workspace_id, type AS "tipo", amount, due_date,
                     category_id, account_id, description, is_paid, created_by, created_at"#,
        workspace_id,
        datos.tipo,
        datos.amount,
        datos.due_date,
        datos.category_id,
        datos.account_id,
        datos.description,
        usuario.id
    )
    .fetch_one(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::PREVISTO_CREADO,
        json!({"previsto_id": fila.id, "type": fila.tipo, "amount": fila.amount}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/previstos — por fecha de vencimiento.
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosPrevistos>,
) -> Result<Json<Vec<Previsto>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Previsto,
        r#"SELECT id, workspace_id, type AS "tipo", amount, due_date,
                  category_id, account_id, description, is_paid, created_by, created_at
           FROM planned_transactions
           WHERE workspace_id = $1
             AND ($2::date IS NULL OR due_date >= $2)
             AND ($3::date IS NULL OR due_date <= $3)
             AND ($4::bool IS NULL OR is_paid = $4)
           ORDER BY due_date"#,
        workspace_id,
        filtros.desde,
        filtros.hasta,
        filtros.pagado
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// PUT /workspaces/:workspace_id/previstos/:id — reemplazo completo.
pub async fn actualizar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<DatosPrevisto>,
) -> Result<Json<Previsto>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_datos(&pool, workspace_id, &datos).await?;

    let fila = sqlx::query_as!(
        Previsto,
        r#"UPDATE planned_transactions
           SET type = $1, amount = $2, due_date = $3, category_id = $4,
               account_id = $5, description = $6
           WHERE id = $7 AND workspace_id = $8
           RETURNING id, workspace_id, type AS "tipo", amount, due_date,
                     category_id, account_id, description, is_paid, created_by, created_at"#,
        datos.tipo,
        datos.amount,
        datos.due_date,
        datos.category_id,
        datos.account_id,
        datos.description,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Previsto no encontrado".to_string()))?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::PREVISTO_EDITADO,
        json!({"previsto_id": fila.id}),
    )
    .await;

    Ok(Json(fila))
}

/// POST /workspaces/:workspace_id/previstos/:id/marcar-pagado
pub async fn marcar_pagado(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Previsto>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let fila = sqlx::query_as!(
        Previsto,
        r#"UPDATE planned_transactions SET is_paid = true
           WHERE id = $1 AND workspace_id = $2
           RETURNING id, workspace_id, type AS "tipo", amount, due_date,
                     category_id, account_id, description, is_paid, created_by, created_at"#,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Previsto no encontrado".to_string()))?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::PREVISTO_PAGADO,
        json!({"previsto_id": fila.id, "amount": fila.amount}),
    )
    .await;

    Ok(Json(fila))
}

/// DELETE /workspaces/:workspace_id/previstos/:id
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let resultado = sqlx::query!(
        "DELETE FROM planned_transactions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado("Previsto no encontrado".to_string()));
    }
    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::PREVISTO_ELIMINADO,
        json!({"previsto_id": id}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
