// =====================================================================
// etiquetas.rs — CRUD de etiquetas del workspace.
// =====================================================================

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::tags::models::{CrearEtiquetaDatos, Etiqueta};

/// POST /workspaces/:workspace_id/etiquetas
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearEtiquetaDatos>,
) -> Result<(StatusCode, Json<Etiqueta>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let resultado = sqlx::query_as!(
        Etiqueta,
        r#"INSERT INTO tags (workspace_id, name)
           VALUES ($1, $2)
           RETURNING id, workspace_id, name, is_active"#,
        workspace_id,
        datos.name.trim()
    )
    .fetch_one(&pool)
    .await;

    match resultado {
        Ok(etiqueta) => {
            auditoria::registrar(
                &pool,
                Some(workspace_id),
                Some(usuario.id),
                acciones::ETIQUETA_CREADA,
                json!({"tag_id": etiqueta.id, "name": etiqueta.name}),
            )
            .await;
            Ok((StatusCode::CREATED, Json(etiqueta)))
        }
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("tags_workspace_name_unique") => {
            Err(AppError::Conflicto(
                "Ya existe una etiqueta con ese nombre en este workspace".to_string(),
            ))
        }
        Err(e) => Err(e.into()),
    }
}

/// GET /workspaces/:workspace_id/etiquetas — solo las activas.
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<Etiqueta>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Etiqueta,
        r#"SELECT id, workspace_id, name, is_active FROM tags
           WHERE workspace_id = $1 AND is_active = true
           ORDER BY name"#,
        workspace_id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// DELETE /workspaces/:workspace_id/etiquetas/:id — borrado lógico: la
/// etiqueta queda en la base (para no romper el historial de
/// transacciones ya etiquetadas) pero deja de listarse y de poder
/// asignarse a nuevas transacciones.
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let resultado = sqlx::query!(
        "UPDATE tags SET is_active = false WHERE id = $1 AND workspace_id = $2 AND is_active = true",
        id,
        workspace_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado("Etiqueta no encontrada".to_string()));
    }
    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::ETIQUETA_ELIMINADA,
        json!({"tag_id": id}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
