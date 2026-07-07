// =====================================================================
// transacciones.rs — Ingresos y gastos variables.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::categorias::validar_categoria;
use crate::accounting::models::{DatosTransaccion, FiltrosTransacciones, Transaccion};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

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

/// Valida los datos comunes a crear/actualizar y, si hay categoría, que
/// sea visible desde el workspace y del mismo tipo que la transacción.
async fn validar_datos(
    pool: &PgPool,
    workspace_id: Uuid,
    datos: &DatosTransaccion,
) -> Result<(), AppError> {
    validar_tipo(&datos.tipo)?;
    validar_monto(datos.amount)?;
    if let Some(category_id) = datos.category_id {
        validar_categoria(pool, category_id, workspace_id, &datos.tipo).await?;
    }
    Ok(())
}

/// POST /workspaces/:workspace_id/transacciones
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<DatosTransaccion>,
) -> Result<(StatusCode, Json<Transaccion>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_datos(&pool, workspace_id, &datos).await?;

    let fila = sqlx::query_as!(
        Transaccion,
        r#"INSERT INTO transactions
               (workspace_id, type, amount, date, category_id, description, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, workspace_id, type AS "tipo", amount, date,
                     category_id, description, created_by, created_at"#,
        workspace_id,
        datos.tipo,
        datos.amount,
        datos.date,
        datos.category_id,
        datos.description,
        usuario.id
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/transacciones — con filtros opcionales.
///
/// Cada filtro usa el patrón `($n::tipo IS NULL OR columna = $n)`: si el
/// cliente no lo manda, la condición se anula sola y no afecta la
/// consulta. Así se arma un solo SQL estático (verificable en tiempo de
/// compilación) sin tener que construir el texto de la query a mano.
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosTransacciones>,
) -> Result<Json<Vec<Transaccion>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let limite = filtros.limite.unwrap_or(50).clamp(1, 200);
    let desplazamiento = filtros.desplazamiento.unwrap_or(0).max(0);

    let filas = sqlx::query_as!(
        Transaccion,
        r#"SELECT id, workspace_id, type AS "tipo", amount, date,
                  category_id, description, created_by, created_at
           FROM transactions
           WHERE workspace_id = $1
             AND ($2::text IS NULL OR type = $2)
             AND ($3::uuid IS NULL OR category_id = $3)
             AND ($4::date IS NULL OR date >= $4)
             AND ($5::date IS NULL OR date <= $5)
             AND ($6::uuid IS NULL OR EXISTS (
                   SELECT 1 FROM transaction_tags tt
                   WHERE tt.transaction_id = transactions.id AND tt.tag_id = $6
                 ))
           ORDER BY date DESC, created_at DESC
           LIMIT $7 OFFSET $8"#,
        workspace_id,
        filtros.tipo,
        filtros.category_id,
        filtros.desde,
        filtros.hasta,
        filtros.tag_id,
        limite,
        desplazamiento
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// GET /workspaces/:workspace_id/transacciones/:id
pub async fn obtener(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Transaccion>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let fila = sqlx::query_as!(
        Transaccion,
        r#"SELECT id, workspace_id, type AS "tipo", amount, date,
                  category_id, description, created_by, created_at
           FROM transactions WHERE id = $1 AND workspace_id = $2"#,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Transacción no encontrada".to_string()))?;

    Ok(Json(fila))
}

/// PUT /workspaces/:workspace_id/transacciones/:id — reemplazo completo.
pub async fn actualizar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<DatosTransaccion>,
) -> Result<Json<Transaccion>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_datos(&pool, workspace_id, &datos).await?;

    let fila = sqlx::query_as!(
        Transaccion,
        r#"UPDATE transactions
           SET type = $1, amount = $2, date = $3, category_id = $4, description = $5
           WHERE id = $6 AND workspace_id = $7
           RETURNING id, workspace_id, type AS "tipo", amount, date,
                     category_id, description, created_by, created_at"#,
        datos.tipo,
        datos.amount,
        datos.date,
        datos.category_id,
        datos.description,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Transacción no encontrada".to_string()))?;

    Ok(Json(fila))
}

/// DELETE /workspaces/:workspace_id/transacciones/:id
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let resultado = sqlx::query!(
        "DELETE FROM transactions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado(
            "Transacción no encontrada".to_string(),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}
