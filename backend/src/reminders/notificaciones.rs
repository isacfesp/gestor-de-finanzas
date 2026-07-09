// =====================================================================
// notificaciones.rs — Listar y marcar como leída una notificación.
// Las notificaciones las crea el motor en background (ver motor.rs);
// aquí solo vive la lectura/escritura vía HTTP.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::reminders::models::{FiltrosNotificaciones, Notificacion};

/// GET /workspaces/:workspace_id/notificaciones?leidas=true|false
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosNotificaciones>,
) -> Result<Json<Vec<Notificacion>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Notificacion,
        r#"SELECT id, type AS "tipo", title, body, reference_id, is_read, created_at
           FROM notifications
           WHERE workspace_id = $1 AND ($2::bool IS NULL OR is_read = $2)
           ORDER BY created_at DESC"#,
        workspace_id,
        filtros.leidas
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// POST /workspaces/:workspace_id/notificaciones/:id/marcar-leida
pub async fn marcar_leida(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Notificacion>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let fila = sqlx::query_as!(
        Notificacion,
        r#"UPDATE notifications SET is_read = true
           WHERE id = $1 AND workspace_id = $2
           RETURNING id, type AS "tipo", title, body, reference_id, is_read, created_at"#,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Notificación no encontrada".to_string()))?;

    Ok(Json(fila))
}
