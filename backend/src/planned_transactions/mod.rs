// Puerta de entrada del módulo planned_transactions: pagos e ingresos
// previstos (eventos únicos a futuro, complementan a subscriptions).
mod handlers;

pub mod models;

use axum::{
    Router,
    routing::{get, post, put},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/previstos", get(handlers::listar).post(handlers::crear))
        .route(
            "/previstos/:id",
            put(handlers::actualizar).delete(handlers::eliminar),
        )
        .route(
            "/previstos/:id/marcar-pagado",
            post(handlers::marcar_pagado),
        )
}
