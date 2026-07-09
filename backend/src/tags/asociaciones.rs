// =====================================================================
// asociaciones.rs — Relación muchos-a-muchos entre transacciones y
// etiquetas (tabla transaction_tags).
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
use crate::tags::models::AgregarEtiquetaDatos;

/// Confirma que la transacción exista y pertenezca al workspace, para
/// no dejar etiquetar (o desetiquetar) recursos ajenos.
async fn validar_transaccion(pool: &PgPool, id: Uuid, workspace_id: Uuid) -> Result<(), AppError> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM transactions
           WHERE id = $1 AND workspace_id = $2 AND is_active = true)"#,
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
            "Transacción no encontrada".to_string(),
        ))
    }
}

/// POST /workspaces/:workspace_id/transacciones/:id/etiquetas
pub async fn agregar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<AgregarEtiquetaDatos>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_transaccion(&pool, id, workspace_id).await?;

    let etiqueta_existe = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM tags
           WHERE id = $1 AND workspace_id = $2 AND is_active = true)"#,
        datos.tag_id,
        workspace_id
    )
    .fetch_one(&pool)
    .await?
    .unwrap_or(false);

    if !etiqueta_existe {
        return Err(AppError::NoEncontrado("Etiqueta no encontrada".to_string()));
    }

    sqlx::query!(
        r#"INSERT INTO transaction_tags (transaction_id, tag_id)
           VALUES ($1, $2)
           ON CONFLICT DO NOTHING"#,
        id,
        datos.tag_id
    )
    .execute(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::ETIQUETA_ASOCIADA,
        json!({"transaction_id": id, "tag_id": datos.tag_id}),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /workspaces/:workspace_id/transacciones/:id/etiquetas/:tag_id
pub async fn quitar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id, tag_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_transaccion(&pool, id, workspace_id).await?;

    let resultado = sqlx::query!(
        "DELETE FROM transaction_tags WHERE transaction_id = $1 AND tag_id = $2",
        id,
        tag_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado(
            "Esa transacción no tiene esa etiqueta".to_string(),
        ));
    }
    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::ETIQUETA_DESASOCIADA,
        json!({"transaction_id": id, "tag_id": tag_id}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
