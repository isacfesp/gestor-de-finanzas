//! Llamadas a `/workspaces/:workspace_id/cuentas` y `/transferencias`.
//! Los structs reflejan 1:1 los de `backend/src/accounts/models.rs`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

// -------------------------------- Cuentas --------------------------------

/// Para `tipo == "credit"`, `balance` es deuda (negativo cuando hay algo
/// usado) y `credit_limit` el límite de la tarjeta; para el resto de
/// los tipos `credit_limit` es `None`.
#[derive(Debug, Clone, Deserialize)]
pub struct Cuenta {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Decimal,
    pub currency: String,
    pub is_active: bool,
    pub credit_limit: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct CrearCuentaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub balance: Option<Decimal>,
    pub currency: Option<&'a str>,
    pub credit_limit: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct ActualizarCuentaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub currency: &'a str,
    pub is_active: bool,
    pub credit_limit: Option<Decimal>,
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

/// GET /workspaces/:workspace_id/transferencias — con rango de fechas
/// opcional (mismo filtro que usa el listado de transacciones, para que
/// la vista combinada de la pestaña Transacciones respete un solo rango).
pub async fn listar_transferencias(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    token: &str,
) -> Result<Vec<Transferencia>, ApiError> {
    let mut partes = Vec::new();
    if let Some(desde) = desde {
        partes.push(format!("desde={desde}"));
    }
    if let Some(hasta) = hasta {
        partes.push(format!("hasta={hasta}"));
    }
    let query = if partes.is_empty() {
        String::new()
    } else {
        format!("?{}", partes.join("&"))
    };
    client::get(
        &format!("/workspaces/{workspace_id}/transferencias{query}"),
        token,
    )
    .await
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
