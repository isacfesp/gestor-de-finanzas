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
/// usado), `credit_limit` el límite de la tarjeta, y `cutoff_day`/
/// `payment_due_day` el día del mes (1-31) en que corta el ciclo y en
/// que vence el pago — recurrente, no una fecha fija. Para el resto de
/// los tipos los tres son `None`.
#[derive(Debug, Clone, Deserialize)]
pub struct Cuenta {
    pub id: Uuid,
    /// Dueño individual de la cuenta: solo él puede operarla. Se usa
    /// para separar "mis cuentas" de las que solo se ven en modo
    /// supervisión (admin/dev).
    pub owner_id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Decimal,
    pub currency: String,
    pub is_active: bool,
    pub credit_limit: Option<Decimal>,
    pub cutoff_day: Option<i16>,
    pub payment_due_day: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct CrearCuentaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub balance: Option<Decimal>,
    pub currency: Option<&'a str>,
    pub credit_limit: Option<Decimal>,
    pub cutoff_day: Option<i16>,
    pub payment_due_day: Option<i16>,
}

#[derive(Debug, Serialize)]
pub struct ActualizarCuentaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub currency: &'a str,
    pub is_active: bool,
    pub credit_limit: Option<Decimal>,
    pub cutoff_day: Option<i16>,
    pub payment_due_day: Option<i16>,
}

/// Fila de `GET /cuentas/alertas-tarjeta`: una tarjeta de crédito cuya
/// fecha de corte o de pago límite cae dentro de la ventana de aviso.
#[derive(Debug, Clone, Deserialize)]
pub struct AlertaTarjeta {
    pub account_id: Uuid,
    pub account_name: String,
    pub currency: String,
    pub balance: Decimal,
    pub credit_limit: Decimal,
    pub cutoff_date: NaiveDate,
    pub payment_due_date: NaiveDate,
}

/// GET /workspaces/:workspace_id/cuentas/alertas-tarjeta?dias=N
pub async fn listar_alertas_tarjeta(
    workspace_id: Uuid,
    dias: Option<i64>,
    token: &str,
) -> Result<Vec<AlertaTarjeta>, ApiError> {
    let query = dias.map(|d| format!("?dias={d}")).unwrap_or_default();
    client::get(
        &format!("/workspaces/{workspace_id}/cuentas/alertas-tarjeta{query}"),
        token,
    )
    .await
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

/// Fila mínima de un miembro del workspace (id + nombre), para
/// resolver "de quién es esta cuenta" en la vista de supervisión.
#[derive(Debug, Clone, Deserialize)]
pub struct MiembroBasico {
    pub user_id: Uuid,
    pub name: String,
}

/// GET /workspaces/:workspace_id/cuentas/miembros — solo admin/dev.
pub async fn listar_miembros(
    workspace_id: Uuid,
    token: &str,
) -> Result<Vec<MiembroBasico>, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/cuentas/miembros"),
        token,
    )
    .await
}

// ------------------------------ Transferencias ------------------------------

/// Fila de listado: igual que el struct de creación pero con el
/// nombre de ambas cuentas ya resuelto por el backend (JOIN) — las
/// cuentas son personales, así que el frontend ya no puede resolverlo
/// cruzando `GET .../cuentas` (un member solo recibe las suyas).
#[derive(Debug, Clone, Deserialize)]
pub struct Transferencia {
    pub from_account_id: Uuid,
    pub from_account_name: String,
    pub to_account_id: Uuid,
    pub to_account_name: String,
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
