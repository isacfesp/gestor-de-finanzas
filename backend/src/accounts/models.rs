// =====================================================================
// models.rs — Structs de datos del módulo accounts.
//
// `type` es palabra reservada en Rust: el campo se llama `tipo` y se
// renombra con #[serde(rename = "type")] para que la API hable en los
// mismos términos que la base de datos (igual que en accounting).
// =====================================================================

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// -------------------------------- Cuentas --------------------------------

/// Cuenta o billetera donde vive físicamente el dinero del workspace.
#[derive(Debug, Serialize)]
pub struct Cuenta {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Decimal,
    pub currency: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// El balance inicial es opcional (por defecto 0): la cuenta nace vacía
/// y solo cambia de saldo mediante transferencias.
#[derive(Debug, Deserialize)]
pub struct CrearCuentaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Option<Decimal>,
    pub currency: Option<String>,
}

/// El balance no se edita aquí a propósito: solo las transferencias
/// mueven dinero entre cuentas, para que el saldo siempre refleje
/// movimientos reales y no una corrección manual silenciosa.
#[derive(Debug, Deserialize)]
pub struct ActualizarCuentaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub currency: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosCuentas {
    pub activas: Option<bool>,
}

// ------------------------------ Transferencias ------------------------------

#[derive(Debug, Serialize)]
pub struct Transferencia {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CrearTransferenciaDatos {
    pub from_account_id: Uuid,
    pub to_account_id: Uuid,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosTransferencias {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    /// Transferencias donde la cuenta participa como origen o destino.
    pub account_id: Option<Uuid>,
}
