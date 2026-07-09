//! Llamadas a `/workspaces/:workspace_id/analytics` (backend
//! `analytics`). El módulo no tiene tablas propias: todo se calcula en
//! runtime sobre `transactions`. Los structs omiten los filtros de la
//! respuesta (`desde`/`hasta`/`month`) — la UI ya sabe qué pidió, mismo
//! criterio de recorte que `api::goals`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Deserialize;
use uuid::Uuid;

use super::client;
use super::error::ApiError;

#[derive(Debug, Clone, Deserialize)]
pub struct FlujoCaja {
    pub income: Decimal,
    pub expense: Decimal,
    pub net: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TasaAhorro {
    pub total_income: Decimal,
    pub goal_income: Decimal,
    pub percentage: Decimal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DistribucionGasto {
    pub category_name: String,
    pub amount: Decimal,
    pub percentage: Decimal,
}

/// GET /workspaces/:workspace_id/analytics/flujo-caja?desde=&hasta=
pub async fn flujo_caja(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    token: &str,
) -> Result<FlujoCaja, ApiError> {
    let ruta = format!(
        "/workspaces/{workspace_id}/analytics/flujo-caja{}",
        query_periodo(desde, hasta)
    );
    client::get(&ruta, token).await
}

/// GET /workspaces/:workspace_id/analytics/tasa-ahorro?month=YYYY-MM-DD
pub async fn tasa_ahorro(
    workspace_id: Uuid,
    month: Option<NaiveDate>,
    token: &str,
) -> Result<TasaAhorro, ApiError> {
    let ruta = match month {
        Some(m) => format!("/workspaces/{workspace_id}/analytics/tasa-ahorro?month={m}"),
        None => format!("/workspaces/{workspace_id}/analytics/tasa-ahorro"),
    };
    client::get(&ruta, token).await
}

/// GET /workspaces/:workspace_id/analytics/distribucion-gastos?desde=&hasta=
pub async fn distribucion_gastos(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    token: &str,
) -> Result<Vec<DistribucionGasto>, ApiError> {
    let ruta = format!(
        "/workspaces/{workspace_id}/analytics/distribucion-gastos{}",
        query_periodo(desde, hasta)
    );
    client::get(&ruta, token).await
}

fn query_periodo(desde: Option<NaiveDate>, hasta: Option<NaiveDate>) -> String {
    let mut partes = Vec::new();
    if let Some(desde) = desde {
        partes.push(format!("desde={desde}"));
    }
    if let Some(hasta) = hasta {
        partes.push(format!("hasta={hasta}"));
    }
    if partes.is_empty() {
        String::new()
    } else {
        format!("?{}", partes.join("&"))
    }
}
