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

/// SVG ya armado por el backend (`charts-rs`, ver
/// `backend/src/analytics/graficos.rs`) — se inyecta directo en el DOM
/// con el atributo `inner_html` de Leptos, sin pasar por `<img>`.
#[derive(Debug, Clone, Deserialize)]
struct GraficoSvg {
    svg: String,
}

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

/// GET /workspaces/:workspace_id/analytics/charts/tendencia?meses=&tema=
///
/// `tema` es "dark"/"light" (`theme::Tema::como_texto()`) — el SVG se
/// arma con esa paleta en el servidor, así que hay que volver a
/// pedirlo si el usuario alterna el tema.
pub async fn tendencia_svg(
    workspace_id: Uuid,
    meses: Option<i64>,
    tema: &str,
    token: &str,
) -> Result<String, ApiError> {
    let mut ruta = format!("/workspaces/{workspace_id}/analytics/charts/tendencia?tema={tema}");
    if let Some(meses) = meses {
        ruta.push_str(&format!("&meses={meses}"));
    }
    client::get::<GraficoSvg>(&ruta, token).await.map(|g| g.svg)
}

/// GET /workspaces/:workspace_id/analytics/charts/flujo-pastel?desde=&hasta=&tema=
pub async fn flujo_pastel_svg(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    tema: &str,
    token: &str,
) -> Result<String, ApiError> {
    let base = query_periodo(desde, hasta);
    let separador = if base.is_empty() { '?' } else { '&' };
    let ruta = format!(
        "/workspaces/{workspace_id}/analytics/charts/flujo-pastel{base}{separador}tema={tema}"
    );
    client::get::<GraficoSvg>(&ruta, token).await.map(|g| g.svg)
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
