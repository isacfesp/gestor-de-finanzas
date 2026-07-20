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
/// la tarjeta— la sube de vuelta hacia 0), `credit_limit` el límite de
/// la tarjeta, y `cutoff_day`/`payment_due_day` el día del mes (1-31)
/// en que corta el ciclo y en que vence el pago — recurrente, no hay
/// que reingresarlo cada mes. Para el resto de los tipos los tres son
/// `None`.
#[derive(Debug, Serialize)]
pub struct Cuenta {
    pub id: Uuid,
    pub workspace_id: Uuid,
    /// Dueño individual de la cuenta: solo él puede operarla (crear
    /// transacciones, editarla, borrarla). Un admin del workspace puede
    /// verla (supervisión) pero no tocarla.
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
    pub created_at: DateTime<Utc>,
}

/// El balance inicial es opcional (por defecto 0): la cuenta nace vacía
/// y solo cambia de saldo mediante transacciones/transferencias.
/// `credit_limit`, `cutoff_day` y `payment_due_day` son obligatorios si
/// `type = "credit"` (lo exige `normalizar_credit_limit`/
/// `normalizar_dias_facturacion`); para el resto de los tipos se ignoran.
#[derive(Debug, Deserialize)]
pub struct CrearCuentaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub balance: Option<Decimal>,
    pub currency: Option<String>,
    pub credit_limit: Option<Decimal>,
    pub cutoff_day: Option<i16>,
    pub payment_due_day: Option<i16>,
}

/// El balance no se edita aquí a propósito: solo las
/// transacciones/transferencias mueven dinero, para que el saldo
/// siempre refleje movimientos reales y no una corrección manual
/// silenciosa. `credit_limit`/`cutoff_day`/`payment_due_day` sí son
/// editables — es lo único que una tarjeta de crédito necesita
/// "editar" más allá de activar/desactivarla.
#[derive(Debug, Deserialize)]
pub struct ActualizarCuentaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
    pub currency: String,
    pub is_active: bool,
    pub credit_limit: Option<Decimal>,
    pub cutoff_day: Option<i16>,
    pub payment_due_day: Option<i16>,
}

/// Fila de `GET /cuentas/alertas-tarjeta`: una tarjeta de crédito cuya
/// fecha de corte o de pago límite cae dentro de la ventana de aviso.
#[derive(Debug, Serialize)]
pub struct AlertaTarjeta {
    pub account_id: Uuid,
    pub account_name: String,
    pub currency: String,
    pub balance: Decimal,
    pub credit_limit: Decimal,
    pub cutoff_date: NaiveDate,
    pub payment_due_date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct FiltroAlertasTarjeta {
    pub dias: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosCuentas {
    pub activas: Option<bool>,
}

/// Fila mínima de un miembro del workspace (id + nombre), para que un
/// admin resuelva "de quién es esta cuenta" en la vista de supervisión.
#[derive(Debug, Serialize)]
pub struct MiembroBasico {
    pub user_id: Uuid,
    pub name: String,
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

/// Fila de listado: igual que `Transferencia` pero con el nombre de
/// ambas cuentas ya resuelto (JOIN con `accounts`, mismo motivo que
/// `accounting::TransaccionListado`: las cuentas son personales, así
/// que un miembro ya no puede resolver el nombre de una cuenta ajena
/// cruzando `GET .../cuentas` en el frontend).
#[derive(Debug, Serialize)]
pub struct TransferenciaListado {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub from_account_id: Uuid,
    pub from_account_name: String,
    pub to_account_id: Uuid,
    pub to_account_name: String,
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
