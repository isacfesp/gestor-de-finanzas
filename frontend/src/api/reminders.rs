//! Llamadas a `/workspaces/:workspace_id/notificaciones` (backend
//! `reminders`). Sin pantalla propia — alimentan la campana del topbar
//! (`components::notificaciones`).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::client;
use super::error::ApiError;

#[derive(Debug, Clone, Deserialize)]
pub struct Notificacion {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// GET /workspaces/:workspace_id/notificaciones?leidas=true|false
pub async fn listar_notificaciones(
    workspace_id: Uuid,
    leidas: Option<bool>,
    token: &str,
) -> Result<Vec<Notificacion>, ApiError> {
    let ruta = match leidas {
        Some(v) => format!("/workspaces/{workspace_id}/notificaciones?leidas={v}"),
        None => format!("/workspaces/{workspace_id}/notificaciones"),
    };
    client::get(&ruta, token).await
}

/// POST /workspaces/:workspace_id/notificaciones/:id/marcar-leida
pub async fn marcar_leida(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<Notificacion, ApiError> {
    client::post_vacio(
        &format!("/workspaces/{workspace_id}/notificaciones/{id}/marcar-leida"),
        token,
    )
    .await
}
