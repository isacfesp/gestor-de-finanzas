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
use crate::accounts::validar_cuenta_propia;
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
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

/// Cuánto (y en qué sentido) mueve el saldo de la cuenta al marcar un
/// previsto como pagado: positivo si es ingreso, negativo si es gasto
/// — mismo criterio que `accounting::transacciones::ajuste_balance`.
fn ajuste_balance(tipo: &str, monto: Decimal) -> Decimal {
    if tipo == "income" { monto } else { -monto }
}

/// Valida tipo, monto y, si vienen, que categoría y cuenta sean
/// visibles desde el workspace — la cuenta, además, debe ser propia
/// de `usuario_id` (un previsto no puede apuntar a la cuenta de otro
/// miembro).
async fn validar_datos(
    pool: &PgPool,
    workspace_id: Uuid,
    usuario_id: Uuid,
    datos: &DatosPrevisto,
) -> Result<(), AppError> {
    validar_tipo(&datos.tipo)?;
    validar_monto(datos.amount)?;
    if let Some(category_id) = datos.category_id {
        validar_categoria(pool, category_id, workspace_id, &datos.tipo).await?;
    }
    if let Some(account_id) = datos.account_id {
        validar_cuenta_propia(pool, account_id, workspace_id, usuario_id).await?;
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
    validar_datos(&pool, workspace_id, usuario.id, &datos).await?;

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
/// Un `member` solo ve los suyos; un `admin`/dev ve todos (supervisión).
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosPrevistos>,
) -> Result<Json<Vec<Previsto>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propios = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let filas = sqlx::query_as!(
        Previsto,
        r#"SELECT id, workspace_id, type AS "tipo", amount, due_date,
                  category_id, account_id, description, is_paid, created_by, created_at
           FROM planned_transactions
           WHERE workspace_id = $1
             AND ($2::date IS NULL OR due_date >= $2)
             AND ($3::date IS NULL OR due_date <= $3)
             AND ($4::bool IS NULL OR is_paid = $4)
             AND ($5::uuid IS NULL OR created_by = $5)
           ORDER BY is_paid, due_date"#,
        workspace_id,
        filtros.desde,
        filtros.hasta,
        filtros.pagado,
        solo_propios
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// PUT /workspaces/:workspace_id/previstos/:id — reemplazo completo.
/// Solo quien lo creó, sin excepción de rol.
pub async fn actualizar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<DatosPrevisto>,
) -> Result<Json<Previsto>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_datos(&pool, workspace_id, usuario.id, &datos).await?;

    let creador = sqlx::query_scalar!(
        "SELECT created_by FROM planned_transactions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Previsto no encontrado".to_string()))?;

    if creador != usuario.id {
        return Err(AppError::Prohibido(
            "Solo quien registró el previsto puede editarlo".to_string(),
        ));
    }

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

    // La transacción que ya generó este previsto (marcar_pagado) queda
    // vinculada por planned_transaction_id — se corrige su categoría
    // también, para que las métricas reflejen el cambio de inmediato.
    // No se propaga monto/cuenta: cambiar eso implicaría re-ajustar
    // saldos ya movidos.
    sqlx::query!(
        "UPDATE transactions SET category_id = $1 WHERE planned_transaction_id = $2 AND is_active = true",
        fila.category_id,
        id
    )
    .execute(&pool)
    .await?;

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
///
/// Si el previsto tiene cuenta asignada, además de cambiar el estado
/// genera el movimiento real: bloquea esa cuenta, ajusta su saldo e
/// inserta la transacción correspondiente — mismo patrón que
/// `accounting::transacciones::crear`, todo en una sola transacción de
/// BD. La fecha de la transacción es `due_date` (cuando el previsto
/// vencía), no la fecha en que se marca pagado, para que caiga en el
/// mes correcto de presupuestos/reportes aunque se marque tarde. Sin
/// cuenta asignada, se mantiene el comportamiento anterior: solo
/// cambia el estado.
pub async fn marcar_pagado(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Previsto>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let mut tx = pool.begin().await?;

    let fila = sqlx::query_as!(
        Previsto,
        r#"UPDATE planned_transactions SET is_paid = true
           WHERE id = $1 AND workspace_id = $2
           RETURNING id, workspace_id, type AS "tipo", amount, due_date,
                     category_id, account_id, description, is_paid, created_by, created_at"#,
        id,
        workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Previsto no encontrado".to_string()))?;

    if fila.created_by != usuario.id {
        return Err(AppError::Prohibido(
            "Solo quien registró el previsto puede marcarlo pagado".to_string(),
        ));
    }

    if let Some(account_id) = fila.account_id {
        let cuenta = sqlx::query_scalar!(
            "SELECT id FROM accounts WHERE id = $1 AND workspace_id = $2 FOR UPDATE",
            account_id,
            workspace_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if cuenta.is_none() {
            return Err(AppError::NoEncontrado("Cuenta no encontrada".to_string()));
        }

        sqlx::query!(
            "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
            ajuste_balance(&fila.tipo, fila.amount),
            account_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"INSERT INTO transactions
                   (workspace_id, type, amount, date, category_id, account_id, description,
                    created_by, planned_transaction_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
            workspace_id,
            fila.tipo,
            fila.amount,
            fila.due_date,
            fila.category_id,
            account_id,
            fila.description,
            usuario.id,
            id
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

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

/// DELETE /workspaces/:workspace_id/previstos/:id — solo quien lo creó.
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let creador = sqlx::query_scalar!(
        "SELECT created_by FROM planned_transactions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Previsto no encontrado".to_string()))?;

    if creador != usuario.id {
        return Err(AppError::Prohibido(
            "Solo quien registró el previsto puede eliminarlo".to_string(),
        ));
    }

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
