//! Llamadas a `/workspaces/:workspace_id/cuentas` y `/transferencias`.
//! Los structs reflejan 1:1 los de `backend/src/accounts/models.rs`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

// -------------------------------- Cuentas --------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Cuenta {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Decimal,
    pub currency: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize)]
pub struct CrearCuentaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub balance: Option<Decimal>,
    pub currency: Option<&'a str>,
}

#[derive(Debug, Serialize)]
pub struct ActualizarCuentaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub currency: &'a str,
    pub is_active: bool,
}

/// GET /workspaces/:workspace_id/cuentas
pub async fn listar_cuentas(workspace_id: Uuid, token: &str) -> Result<Vec<Cuenta>, ApiError> {
    client::get(&format!("/workspaces/{workspace_id}/cuentas"), token).await
}

/// POST /workspaces/:workspace_id/cuentas
pub async fn crear_cuenta(
    workspace_id: Uuid,
    datos: &CrearCuentaDatos<'_>,
    token: &str,
) -> Result<Cuenta, ApiError> {
    client::post(&format!("/workspaces/{workspace_id}/cuentas"), datos, token).await
}

/// PUT /workspaces/:workspace_id/cuentas/:id
pub async fn actualizar_cuenta(
    workspace_id: Uuid,
    id: Uuid,
    datos: &ActualizarCuentaDatos<'_>,
    token: &str,
) -> Result<Cuenta, ApiError> {
    client::put(
        &format!("/workspaces/{workspace_id}/cuentas/{id}"),
        datos,
        token,
    )
    .await
}

// Borrar una cuenta no está expuesto en esta primera pasada del módulo:
// el flujo previsto es desactivarla (`is_active`) vía `actualizar_cuenta`,
// no borrarla — evita lidiar con la FK de `transactions.account_id`.

// ------------------------------ Transferencias ------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Transferencia {
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CrearTransferenciaDatos<'a> {
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub description: Option<&'a str>,
}

/// GET /workspaces/:workspace_id/transferencias
pub async fn listar_transferencias(
    workspace_id: Uuid,
    token: &str,
) -> Result<Vec<Transferencia>, ApiError> {
    client::get(&format!("/workspaces/{workspace_id}/transferencias"), token).await
}

/// POST /workspaces/:workspace_id/transferencias
pub async fn crear_transferencia(
    workspace_id: Uuid,
    datos: &CrearTransferenciaDatos<'_>,
    token: &str,
) -> Result<Transferencia, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/transferencias"),
        datos,
        token,
    )
    .await
}
