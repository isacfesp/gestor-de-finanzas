// Puerta de entrada del módulo analytics: métricas agregadas en runtime
// sobre `transactions`, sin tablas propias (ver docs/database.md).
mod comun;
mod graficos;
mod metricas;

pub mod models;

use axum::{Router, routing::get};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/analytics/flujo-caja", get(metricas::flujo_caja))
        .route("/analytics/ahorro-neto", get(metricas::ahorro_neto))
        .route("/analytics/tasa-ahorro", get(metricas::tasa_ahorro))
        .route(
            "/analytics/distribucion-gastos",
            get(metricas::distribucion_gastos),
        )
        .route("/analytics/charts/tendencia", get(graficos::tendencia))
        .route(
            "/analytics/charts/flujo-pastel",
            get(graficos::flujo_pastel),
        )
}
