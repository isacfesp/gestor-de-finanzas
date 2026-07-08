//! Llamadas a `/workspaces/:workspace_id/etiquetas` y a la asociación
//! etiqueta↔transacción. Structs 1:1 con `backend/src/tags/models.rs`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

#[derive(Debug, Clone, Deserialize)]
pub struct Etiqueta {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CrearEtiquetaDatos<'a> {
    pub name: &'a str,
}

#[derive(Debug, Serialize)]
struct AgregarEtiquetaDatos {
    tag_id: Uuid,
}

/// GET /workspaces/:workspace_id/etiquetas
pub async fn listar_etiquetas(workspace_id: Uuid, token: &str) -> Result<Vec<Etiqueta>, ApiError> {
    client::get(&format!("/workspaces/{workspace_id}/etiquetas"), token).await
}

/// POST /workspaces/:workspace_id/etiquetas
pub async fn crear_etiqueta(
    workspace_id: Uuid,
    datos: &CrearEtiquetaDatos<'_>,
    token: &str,
) -> Result<Etiqueta, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/etiquetas"),
        datos,
        token,
    )
    .await
}

/// POST /workspaces/:workspace_id/transacciones/:id/etiquetas — asocia
/// una etiqueta ya existente a una transacción.
pub async fn agregar_etiqueta_a_transaccion(
    workspace_id: Uuid,
    transaccion_id: Uuid,
    tag_id: Uuid,
    token: &str,
) -> Result<(), ApiError> {
    client::post_sin_respuesta(
        &format!("/workspaces/{workspace_id}/transacciones/{transaccion_id}/etiquetas"),
        &AgregarEtiquetaDatos { tag_id },
        token,
    )
    .await
}
