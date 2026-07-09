//! Llamadas a `/workspaces/:workspace_id/metas` (backend `goals`).
//! Los structs reflejan los campos de `backend/src/goals/models.rs`
//! que la UI necesita (se omiten `workspace_id`/`created_at`, igual
//! que `api::agenda::Previsto` con los suyos).

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub id: Uuid,
    pub name: String,
    pub target_amount: Decimal,
    pub current_amount: Decimal,
    pub deadline: NaiveDate,
    pub is_completed: bool,
}

/// Mismo cuerpo para crear y editar (el backend usa
/// `CrearMetaDatos`/`ActualizarMetaDatos`, con forma idéntica).
#[derive(Debug, Serialize)]
pub struct DatosMeta<'a> {
    pub name: &'a str,
    pub target_amount: Decimal,
    pub deadline: NaiveDate,
}

/// GET /workspaces/:workspace_id/metas?completadas=true|false
pub async fn listar_metas(
    workspace_id: Uuid,
    completadas: Option<bool>,
    token: &str,
) -> Result<Vec<Meta>, ApiError> {
    let ruta = match completadas {
        Some(valor) => format!("/workspaces/{workspace_id}/metas?completadas={valor}"),
        None => format!("/workspaces/{workspace_id}/metas"),
    };
    client::get(&ruta, token).await
}

/// POST /workspaces/:workspace_id/metas
pub async fn crear_meta(
    workspace_id: Uuid,
    datos: &DatosMeta<'_>,
    token: &str,
) -> Result<Meta, ApiError> {
    client::post(&format!("/workspaces/{workspace_id}/metas"), datos, token).await
}

/// PUT /workspaces/:workspace_id/metas/:id — no toca current_amount ni
/// is_completed (solo cambian al vincular un aporte).
pub async fn actualizar_meta(
    workspace_id: Uuid,
    id: Uuid,
    datos: &DatosMeta<'_>,
    token: &str,
) -> Result<Meta, ApiError> {
    client::put(
        &format!("/workspaces/{workspace_id}/metas/{id}"),
        datos,
        token,
    )
    .await
}

/// DELETE /workspaces/:workspace_id/metas/:id — 409 si tiene aportes
/// vinculados; el mensaje ya viene legible en `ApiError::Servidor`.
pub async fn eliminar_meta(workspace_id: Uuid, id: Uuid, token: &str) -> Result<(), ApiError> {
    client::delete(&format!("/workspaces/{workspace_id}/metas/{id}"), token).await
}

/// `tipo` en `None` deja que el backend use su default ('income').
#[derive(Debug, Serialize)]
pub struct DatosAporte<'a> {
    pub amount: Decimal,
    #[serde(rename = "type")]
    pub tipo: Option<&'a str>,
    pub date: NaiveDate,
    pub description: Option<&'a str>,
}

/// POST /workspaces/:workspace_id/metas/:id/aportes — devuelve la Meta
/// ya con current_amount/is_completed actualizados.
pub async fn registrar_aporte(
    workspace_id: Uuid,
    id: Uuid,
    datos: &DatosAporte<'_>,
    token: &str,
) -> Result<Meta, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/metas/{id}/aportes"),
        datos,
        token,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct Aporte {
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub description: Option<String>,
    /// Quién de los miembros del workspace registró este aporte — la
    /// meta es colaborativa entre varias personas.
    pub created_by_name: String,
}

/// GET /workspaces/:workspace_id/metas/:id/aportes
pub async fn listar_aportes(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<Vec<Aporte>, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/metas/{id}/aportes"),
        token,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProgresoMeta {
    pub target_amount: Decimal,
    pub current_amount: Decimal,
    pub remaining_amount: Decimal,
    pub percentage: Decimal,
}

/// GET /workspaces/:workspace_id/metas/:id/progreso
pub async fn progreso_meta(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<ProgresoMeta, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/metas/{id}/progreso"),
        token,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProyeccionMeta {
    pub periodo: String,
    pub periodos_restantes: i64,
    pub aporte_necesario: Decimal,
}

/// GET /workspaces/:workspace_id/metas/:id/proyeccion?periodo=weekly|monthly
pub async fn proyeccion_meta(
    workspace_id: Uuid,
    id: Uuid,
    periodo: &str,
    token: &str,
) -> Result<ProyeccionMeta, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/metas/{id}/proyeccion?periodo={periodo}"),
        token,
    )
    .await
}
