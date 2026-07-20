// =====================================================================
// models.rs — Structs de datos del módulo investments.
// =====================================================================

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Inversión registrada (SOFIPO). El rendimiento y el ISR al
/// vencimiento se calculan al vuelo (ver calculos.rs); el rendimiento
/// diario ya acumulado se guarda en investment_accruals.
#[derive(Debug, Serialize)]
pub struct Inversion {
    pub id: Uuid,
    pub workspace_id: Uuid,
    /// Dueño individual: solo él la crea/edita/elimina. Un admin/dev
    /// del workspace puede verla (supervisión) pero no operarla.
    pub owner_id: Uuid,
    pub name: String,
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    /// Tasa de ISR propia de esta inversión (porcentaje anual, no
    /// fracción) — editable, ya no una constante global.
    pub isr_annual_rate: Decimal,
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
    pub isr_annual_rate: Decimal,
    pub interest_type: String,
    pub start_date: NaiveDate,
    pub term_days: i32,
}

/// Reemplazo completo de los campos editables de una inversión. Cambiar
/// `principal`/`start_date`/`term_days` no recalcula retroactivamente
/// los accruals ya guardados en investment_accruals — el job diario
/// sigue calculando hacia adelante con los nuevos valores desde la
/// fecha del cambio.
#[derive(Debug, Deserialize)]
pub struct ActualizarInversionDatos {
    pub name: String,
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    pub isr_annual_rate: Decimal,
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
/// `isr_annual_rate` es opcional porque el simulador no representa una
/// inversión real con tasa propia — si se omite, usa la tasa sugerida.
#[derive(Debug, Deserialize)]
pub struct SimularInversionDatos {
    pub principal: Decimal,
    pub gat_annual_rate: Decimal,
    #[serde(default = "crate::investments::calculos::tasa_retencion_isr_anual")]
    pub isr_annual_rate: Decimal,
    pub interest_type: String,
    pub term_days: i32,
}

/// Agregado de todas las inversiones activas del workspace (o solo las
/// propias, para un member) — lo consume el Dashboard.
#[derive(Debug, Serialize)]
pub struct ResumenAhorroInversiones {
    pub principal_invertido: Decimal,
    pub gross_yield_acumulado: Decimal,
    pub isr_acumulado: Decimal,
    pub net_yield_acumulado: Decimal,
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
