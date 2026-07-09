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
///
/// Para `type = "credit"`, `balance` representa deuda (negativo cuando
/// hay algo usado: un gasto la baja, un pago —vía transferencia hacia
/// la tarjeta— la sube de vuelta hacia 0) y `credit_limit` el límite de
/// la tarjeta. Para el resto de los tipos `credit_limit` es `None`.
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
    pub credit_limit: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

/// El balance inicial es opcional (por defecto 0): la cuenta nace vacía
/// y solo cambia de saldo mediante transacciones/transferencias.
/// `credit_limit` es obligatorio si `type = "credit"` (lo exige
/// `validar_credit_limit`); para el resto de los tipos se ignora.
#[derive(Debug, Deserialize)]
pub struct CrearCuentaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Option<Decimal>,
    pub currency: Option<String>,
    pub credit_limit: Option<Decimal>,
}

/// El balance no se edita aquí a propósito: solo las
/// transacciones/transferencias mueven dinero, para que el saldo
/// siempre refleje movimientos reales y no una corrección manual
/// silenciosa. `credit_limit` sí es editable — es la única razón por la
/// que una tarjeta de crédito necesita "editarse" más allá de
/// activar/desactivarla (subir o bajar el límite).
#[derive(Debug, Deserialize)]
pub struct ActualizarCuentaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub currency: String,
    pub is_active: bool,
    pub credit_limit: Option<Decimal>,
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
