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

/// Dinero nuevo aportado a metas en el rango (aportes − retiros), en
/// monto absoluto — a diferencia de `TasaAhorro`, que es un %.
#[derive(Debug, Clone, Deserialize)]
pub struct AhorroNeto {
    pub aportado: Decimal,
    pub retirado: Decimal,
    pub neto: Decimal,
}

/// GET /workspaces/:workspace_id/analytics/flujo-caja?desde=&hasta=&user_id=
///
/// `user_id` solo tiene efecto si quien pregunta es un dev global —
/// cualquier otro usuario recibe sus propias métricas sin importar lo
/// que mande aquí (lo decide el backend, ver
/// `analytics::comun::resolver_filtro_usuario`). `None` desde un dev
/// significa "todo el workspace".
pub async fn flujo_caja(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    user_id: Option<Uuid>,
    token: &str,
) -> Result<FlujoCaja, ApiError> {
    let mut ruta = format!(
        "/workspaces/{workspace_id}/analytics/flujo-caja{}",
        query_periodo(desde, hasta)
    );
    agregar_param(&mut ruta, "user_id", user_id);
    client::get(&ruta, token).await
}

/// GET /workspaces/:workspace_id/analytics/tasa-ahorro?month=YYYY-MM-DD&user_id=
pub async fn tasa_ahorro(
    workspace_id: Uuid,
    month: Option<NaiveDate>,
    user_id: Option<Uuid>,
    token: &str,
) -> Result<TasaAhorro, ApiError> {
    let mut ruta = match month {
        Some(m) => format!("/workspaces/{workspace_id}/analytics/tasa-ahorro?month={m}"),
        None => format!("/workspaces/{workspace_id}/analytics/tasa-ahorro"),
    };
    agregar_param(&mut ruta, "user_id", user_id);
    client::get(&ruta, token).await
}

/// GET /workspaces/:workspace_id/analytics/ahorro-neto?desde=&hasta=&user_id=
pub async fn ahorro_neto(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    user_id: Option<Uuid>,
    token: &str,
) -> Result<AhorroNeto, ApiError> {
    let mut ruta = format!(
        "/workspaces/{workspace_id}/analytics/ahorro-neto{}",
        query_periodo(desde, hasta)
    );
    agregar_param(&mut ruta, "user_id", user_id);
    client::get(&ruta, token).await
}

/// GET /workspaces/:workspace_id/analytics/charts/tendencia?tema=&user_id=&granularidad=
///
/// `tema` es "dark"/"light" (`theme::Tema::como_texto()`) — el SVG se
/// arma con esa paleta en el servidor, así que hay que volver a
/// pedirlo si el usuario alterna el tema. `granularidad` fija tanto el
/// rango como la agrupación: "semana" (semana en curso, día por día),
/// "mes" (mes en curso, semana por semana, por defecto si se omite) o
/// "año" (últimos 12 meses, mes por mes).
pub async fn tendencia_svg(
    workspace_id: Uuid,
    tema: &str,
    user_id: Option<Uuid>,
    granularidad: &str,
    token: &str,
) -> Result<String, ApiError> {
    let mut ruta = format!(
        "/workspaces/{workspace_id}/analytics/charts/tendencia?tema={tema}&granularidad={granularidad}"
    );
    agregar_param(&mut ruta, "user_id", user_id);
    client::get::<GraficoSvg>(&ruta, token).await.map(|g| g.svg)
}

/// GET /workspaces/:workspace_id/analytics/charts/flujo-pastel?desde=&hasta=&tema=&user_id=
pub async fn flujo_pastel_svg(
    workspace_id: Uuid,
    desde: Option<NaiveDate>,
    hasta: Option<NaiveDate>,
    tema: &str,
    user_id: Option<Uuid>,
    token: &str,
) -> Result<String, ApiError> {
    let base = query_periodo(desde, hasta);
    let separador = if base.is_empty() { '?' } else { '&' };
    let mut ruta = format!(
        "/workspaces/{workspace_id}/analytics/charts/flujo-pastel{base}{separador}tema={tema}"
    );
    agregar_param(&mut ruta, "user_id", user_id);
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

/// Anexa `&nombre=valor` (o `?` si `ruta` todavía no tiene query
/// string) cuando `valor` es `Some` — no hace nada si es `None`.
fn agregar_param(ruta: &mut String, nombre: &str, valor: Option<Uuid>) {
    if let Some(valor) = valor {
        let separador = if ruta.contains('?') { '&' } else { '?' };
        ruta.push_str(&format!("{separador}{nombre}={valor}"));
    }
}
