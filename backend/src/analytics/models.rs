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

/// `tema` selecciona la paleta claro/oscuro en el momento de generar el
/// SVG (ver `graficos.rs`) — el servidor no puede reaccionar a un
/// cambio de tema hecho después en el cliente, así que el frontend
/// vuelve a pedir el gráfico cuando el usuario alterna el tema.
#[derive(Debug, Deserialize)]
pub struct FiltroTendencia {
    pub tema: Option<String>,
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
    pub tema: Option<String>,
    pub user_id: Option<Uuid>,
}

/// SVG ya armado, como texto — el frontend lo inyecta directo con
/// `inner_html`, sin pasar por `<img>`/Blob.
#[derive(Debug, Serialize)]
pub struct GraficoSvg {
    pub svg: String,
}
