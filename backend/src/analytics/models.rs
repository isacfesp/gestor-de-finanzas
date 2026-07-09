// =====================================================================
// models.rs — Structs de datos del módulo analytics.
//
// El módulo no tiene tablas propias: todo se calcula en runtime sobre
// `transactions` (ver docs/database.md).
// =====================================================================

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct FiltroPeriodo {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct FiltroMes {
    pub month: Option<NaiveDate>,
}

/// Ingresos − egresos en el período pedido (o en todo el historial si
/// no se acota).
#[derive(Debug, Serialize)]
pub struct FlujoCaja {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub income: Decimal,
    pub expense: Decimal,
    pub net: Decimal,
}

/// % del ingreso del mes que se destinó a metas (transacciones con
/// `goal_id`), no a cuentas de tipo `savings` — son conceptos
/// distintos en este proyecto (ver `goals::aportes`).
#[derive(Debug, Serialize)]
pub struct TasaAhorro {
    pub month: NaiveDate,
    pub total_income: Decimal,
    pub goal_income: Decimal,
    pub percentage: Decimal,
}

/// Una fila de la distribución de gastos por categoría.
/// `category_id` es `None` para el bucket "Sin categoría".
#[derive(Debug, Serialize)]
pub struct DistribucionGasto {
    pub category_id: Option<Uuid>,
    pub category_name: String,
    pub amount: Decimal,
    pub percentage: Decimal,
}
