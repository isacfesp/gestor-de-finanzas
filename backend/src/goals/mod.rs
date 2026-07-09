// Puerta de entrada del módulo goals: metas de ahorro, sus aportes y
// el cálculo de progreso/proyección.
mod aportes;
mod metas;

pub mod models;

use axum::{
    Router,
    routing::{get, put},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/metas", get(metas::listar).post(metas::crear))
        .route("/metas/:id", put(metas::actualizar).delete(metas::eliminar))
        .route(
            "/metas/:id/aportes",
            get(aportes::listar_aportes).post(aportes::vincular),
        )
        .route("/metas/:id/progreso", get(aportes::progreso))
        .route("/metas/:id/proyeccion", get(aportes::proyeccion))
}
