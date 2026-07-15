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
    /// Solo tiene efecto si quien pregunta es un dev global (ver
    /// `comun::resolver_filtro_usuario`): cualquier otro usuario
    /// siempre recibe sus propias métricas, ignorando este campo.
    pub user_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct FiltroMes {
    pub month: Option<NaiveDate>,
    pub user_id: Option<Uuid>,
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

/// Dinero nuevo aportado a metas en el rango: aportes (ingreso) menos
/// retiros (egreso) con `goal_id`. A diferencia de `TasaAhorro` (que
/// es un %, atado al mes en curso), esto es un monto absoluto sobre un
/// rango de fechas libre.
#[derive(Debug, Serialize)]
pub struct AhorroNeto {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub aportado: Decimal,
    pub retirado: Decimal,
    pub neto: Decimal,
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

#[derive(Debug, Deserialize)]
pub struct FiltroTendencia {
    pub user_id: Option<Uuid>,
    /// "semana" (la semana en curso, día por día) | "mes" (el mes en
    /// curso, semana por semana) | "año" (últimos 12 meses, mes por
    /// mes) — por defecto "mes". Cualquier otro valor cae también en
    /// "mes" (ver `graficos::parsear_vista`).
    pub granularidad: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FiltroFlujoPastel {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub user_id: Option<Uuid>,
}

/// Un punto de la gráfica de tendencia. `net` (ingresos − egresos del
/// período) puede ser negativo — significa que ese período se gastó
/// más de lo que entró, es decir, se cayó en deuda.
#[derive(Debug, Serialize)]
pub struct PuntoTendencia {
    pub etiqueta: String,
    pub income: Decimal,
    pub expense: Decimal,
    pub net: Decimal,
}

#[derive(Debug, Serialize)]
pub struct DatosTendencia {
    pub puntos: Vec<PuntoTendencia>,
}

/// Una rebanada del pastel de una categoría. `percentage` es relativo
/// al total de su propio tipo (todos los ingresos o todos los gastos
/// del período, cada uno por separado) — así las rebanadas de cada
/// pastel suman 100%, en vez de repartirse sobre el combinado de
/// ambos tipos como pasaba antes.
#[derive(Debug, Serialize)]
pub struct RebanadaPastel {
    pub category_id: Option<Uuid>,
    pub category_name: String,
    pub amount: Decimal,
    pub percentage: Decimal,
}

#[derive(Debug, Serialize)]
pub struct DatosFlujoPastel {
    pub ingresos: Vec<RebanadaPastel>,
    pub gastos: Vec<RebanadaPastel>,
}
