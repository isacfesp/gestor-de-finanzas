//! Llamadas a `/workspaces/:workspace_id/inversiones` (backend
//! `investments`). `DesgloseRendimiento` es la misma forma para el
//! simulador (`simular_inversion`, sin persistir) y para la
//! proyección de una inversión ya registrada (`proyeccion_inversion`);
//! el frontend la comparte entre ambos con `TarjetaDesglose`
//! (`pages/modulos/inversiones/desglose.rs`).
//!
//! No hay `obtener_inversion` ni `actualizar_inversion`: el backend no
//! expone PUT para este recurso, y ninguna pantalla necesita volver a
//! pedir una inversión que ya llegó con el listado.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

/// `interest_type` es un `String` validado por el backend contra estos
/// 2 valores — no existe como enum serde, se replica igual (mismo
/// patrón que `PERIODICIDADES` en `api/agenda.rs`).
pub const TIPOS_INTERES: [(&str, &str); 2] = [("simple", "Simple"), ("compound", "Compuesto")];

pub fn etiqueta_tipo_interes(valor: &str) -> &'static str {
    TIPOS_INTERES
        .iter()
        .find(|(v, _)| *v == valor)
        .map(|(_, etiqueta)| *etiqueta)
        .unwrap_or("Desconocido")
}

#[derive(Debug, Clone, Deserialize)]
pub struct Inversion {
    pub id: Uuid,
    pub name: String,
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct DatosInversion<'a> {
    pub name: &'a str,
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: &'a str,
    pub start_date: NaiveDate,
    pub term_days: i32,
}

/// GET /workspaces/:workspace_id/inversiones — siempre trae todo; el
/// estado "vigente/vencida" se calcula en el cliente (ver
/// `inversiones_tab::estado_de`), porque el filtro `activas` del
/// backend es sobre la columna manual `is_active`, no sobre `end_date`.
pub async fn listar_inversiones(
    workspace_id: Uuid,
    token: &str,
) -> Result<Vec<Inversion>, ApiError> {
    client::get(&format!("/workspaces/{workspace_id}/inversiones"), token).await
}

/// POST /workspaces/:workspace_id/inversiones
pub async fn crear_inversion(
    workspace_id: Uuid,
    datos: &DatosInversion<'_>,
    token: &str,
) -> Result<Inversion, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/inversiones"),
        datos,
        token,
    )
    .await
}

/// DELETE /workspaces/:workspace_id/inversiones/:id — borra en cascada
/// investment_yields, sin más advertencia del backend.
pub async fn eliminar_inversion(workspace_id: Uuid, id: Uuid, token: &str) -> Result<(), ApiError> {
    client::delete(
        &format!("/workspaces/{workspace_id}/inversiones/{id}"),
        token,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct DesgloseRendimiento {
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: String,
    pub term_days: i32,
    pub gross_yield: Decimal,
    pub isr_amount: Decimal,
    pub net_yield: Decimal,
    pub maturity_amount: Decimal,
}

/// GET /workspaces/:workspace_id/inversiones/:id/proyeccion
pub async fn proyeccion_inversion(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<DesgloseRendimiento, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/inversiones/{id}/proyeccion"),
        token,
    )
    .await
}

#[derive(Debug, Serialize)]
pub struct DatosSimulacion {
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: String,
    pub term_days: i32,
}

/// POST /workspaces/:workspace_id/inversiones/simular — no persiste nada.
pub async fn simular_inversion(
    workspace_id: Uuid,
    datos: &DatosSimulacion,
    token: &str,
) -> Result<DesgloseRendimiento, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/inversiones/simular"),
        datos,
        token,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct Rendimiento {
    pub yield_amount: Decimal,
    pub yield_date: NaiveDate,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DatosRendimiento<'a> {
    pub yield_amount: Decimal,
    pub yield_date: NaiveDate,
    pub notes: Option<&'a str>,
}

/// GET /workspaces/:workspace_id/inversiones/:id/rendimientos
pub async fn listar_rendimientos(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<Vec<Rendimiento>, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/inversiones/{id}/rendimientos"),
        token,
    )
    .await
}

/// POST /workspaces/:workspace_id/inversiones/:id/rendimientos
pub async fn registrar_rendimiento(
    workspace_id: Uuid,
    id: Uuid,
    datos: &DatosRendimiento<'_>,
    token: &str,
) -> Result<Rendimiento, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/inversiones/{id}/rendimientos"),
        datos,
        token,
    )
    .await
}
