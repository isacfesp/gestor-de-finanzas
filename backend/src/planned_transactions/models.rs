// =====================================================================
// models.rs — Structs de datos del módulo planned_transactions.
// =====================================================================

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Pago o ingreso previsto: un evento único a futuro (a diferencia de
/// las suscripciones, que son recurrentes).
#[derive(Debug, Serialize)]
pub struct Previsto {
    pub id: Uuid,
    pub workspace_id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_paid: bool,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Se usa tanto para crear como para reemplazar un previsto existente
/// (PUT): en ambos casos se exigen todos los campos, salvo `is_paid`
/// que se maneja aparte con el endpoint de marcar-pagado.
#[derive(Debug, Deserialize)]
pub struct DatosPrevisto {
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosPrevistos {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub pagado: Option<bool>,
}
