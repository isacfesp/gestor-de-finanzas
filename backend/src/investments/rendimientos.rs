// =====================================================================
// rendimientos.rs — Historial de rendimientos reales acreditados
// (investment_yields). Distinto de la proyección: esto es lo que
// realmente pagó la SOFIPO, no una estimación.
// =====================================================================

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::investments::models::{CrearRendimientoDatos, Rendimiento};

/// Confirma que la inversión exista y pertenezca al workspace, para no
/// registrar (ni listar) rendimientos de una inversión ajena.
async fn validar_inversion(pool: &PgPool, id: Uuid, workspace_id: Uuid) -> Result<(), AppError> {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM investments WHERE id = $1 AND workspace_id = $2)",
        id,
        workspace_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);

    if existe {
        Ok(())
    } else {
        Err(AppError::NoEncontrado(
            "Inversión no encontrada".to_string(),
        ))
    }
}

/// POST /workspaces/:workspace_id/inversiones/:id/rendimientos
pub async fn registrar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<CrearRendimientoDatos>,
) -> Result<(StatusCode, Json<Rendimiento>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_inversion(&pool, id, workspace_id).await?;

    if datos.yield_amount <= Decimal::ZERO {
        return Err(AppError::NoProcesable(
            "El rendimiento debe ser mayor a cero".to_string(),
        ));
    }

    let fila = sqlx::query_as!(
        Rendimiento,
        r#"INSERT INTO investment_yields (investment_id, yield_amount, yield_date, notes)
           VALUES ($1, $2, $3, $4)
           RETURNING id, investment_id, yield_amount, yield_date, notes, created_at"#,
        id,
        datos.yield_amount,
        datos.yield_date,
        datos.notes
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/inversiones/:id/rendimientos
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Rendimiento>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_inversion(&pool, id, workspace_id).await?;

    let filas = sqlx::query_as!(
        Rendimiento,
        r#"SELECT id, investment_id, yield_amount, yield_date, notes, created_at
           FROM investment_yields
           WHERE investment_id = $1
           ORDER BY yield_date DESC"#,
        id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}
