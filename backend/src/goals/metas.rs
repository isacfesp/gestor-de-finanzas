// =====================================================================
// metas.rs — CRUD de metas de ahorro.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::goals::models::{ActualizarMetaDatos, CrearMetaDatos, FiltrosMetas, Meta};

fn validar_monto_objetivo(monto: Decimal) -> Result<(), AppError> {
    if monto > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El monto objetivo debe ser mayor a cero".to_string(),
        ))
    }
}

/// POST /workspaces/:workspace_id/metas
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearMetaDatos>,
) -> Result<(StatusCode, Json<Meta>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_monto_objetivo(datos.target_amount)?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let fila = sqlx::query_as!(
        Meta,
        r#"INSERT INTO goals (workspace_id, name, target_amount, deadline)
           VALUES ($1, $2, $3, $4)
           RETURNING id, workspace_id, name, target_amount, current_amount,
                     deadline, is_completed, created_at"#,
        workspace_id,
        datos.name.trim(),
        datos.target_amount,
        datos.deadline
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/metas?completadas=true|false
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosMetas>,
) -> Result<Json<Vec<Meta>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Meta,
        r#"SELECT id, workspace_id, name, target_amount, current_amount,
                  deadline, is_completed, created_at
           FROM goals
           WHERE workspace_id = $1
             AND ($2::bool IS NULL OR is_completed = $2)
           ORDER BY deadline"#,
        workspace_id,
        filtros.completadas
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// PUT /workspaces/:workspace_id/metas/:id — no toca current_amount ni
/// is_completed (esos solo cambian al vincular un aporte).
pub async fn actualizar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<ActualizarMetaDatos>,
) -> Result<Json<Meta>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_monto_objetivo(datos.target_amount)?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let fila = sqlx::query_as!(
        Meta,
        r#"UPDATE goals
           SET name = $1, target_amount = $2, deadline = $3,
               is_completed = (current_amount >= $2)
           WHERE id = $4 AND workspace_id = $5
           RETURNING id, workspace_id, name, target_amount, current_amount,
                     deadline, is_completed, created_at"#,
        datos.name.trim(),
        datos.target_amount,
        datos.deadline,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Meta no encontrada".to_string()))?;

    Ok(Json(fila))
}

/// DELETE /workspaces/:workspace_id/metas/:id
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let resultado = sqlx::query!(
        "DELETE FROM goals WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await;

    match resultado {
        Ok(r) if r.rows_affected() == 0 => {
            Err(AppError::NoEncontrado("Meta no encontrada".to_string()))
        }
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        // La meta está referenciada por transacciones con goal_id.
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23503") => Err(
            AppError::Conflicto("La meta está en uso, no se puede eliminar".to_string()),
        ),
        Err(e) => Err(e.into()),
    }
}
