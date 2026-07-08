// =====================================================================
// models.rs — Structs de datos del módulo investments.
// =====================================================================

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Inversión registrada (SOFIPO): el rendimiento y el ISR no se
/// almacenan aquí, se calculan al vuelo (ver calculos.rs).
#[derive(Debug, Serialize)]
pub struct Inversion {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: String,
    pub start_date: NaiveDate,
    pub term_days: i32,
    pub end_date: NaiveDate,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

/// `end_date` no se pide: se calcula como `start_date + term_days`.
#[derive(Debug, Deserialize)]
pub struct CrearInversionDatos {
    pub name: String,
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: String,
    pub start_date: NaiveDate,
    pub term_days: i32,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosInversiones {
    pub activas: Option<bool>,
}

/// Body del simulador: los mismos datos financieros que una inversión,
/// pero sin `name` ni `start_date` porque nunca se persiste.
#[derive(Debug, Deserialize)]
pub struct SimularInversionDatos {
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub interest_type: String,
    pub term_days: i32,
}

/// Desglose de rendimiento bruto, ISR retenido y neto. La usan tanto
/// el simulador (datos sueltos) como la proyección de una inversión ya
/// registrada (por eso no incluye `investment_id`; ese campo lo añade
/// el endpoint de proyección por separado si hace falta).
#[derive(Debug, Serialize)]
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

// --------------------------- Rendimientos reales ---------------------------

/// Rendimiento real acreditado a una inversión (investment_yields).
#[derive(Debug, Serialize)]
pub struct Rendimiento {
    pub id: Uuid,
    pub investment_id: Uuid,
    pub yield_amount: Decimal,
    pub yield_date: NaiveDate,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CrearRendimientoDatos {
    pub yield_amount: Decimal,
    pub yield_date: NaiveDate,
    pub notes: Option<String>,
}
