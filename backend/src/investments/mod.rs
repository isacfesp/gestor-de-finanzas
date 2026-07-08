// Puerta de entrada del módulo investments: inversiones, su proyección
// de rendimiento/ISR, el simulador y el historial de rendimientos
// reales acreditados.
mod calculos;
mod inversiones;
mod rendimientos;

pub mod models;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/inversiones",
            get(inversiones::listar).post(inversiones::crear),
        )
        .route("/inversiones/simular", post(inversiones::simular))
        .route(
            "/inversiones/:id",
            get(inversiones::obtener).delete(inversiones::eliminar),
        )
        .route("/inversiones/:id/proyeccion", get(inversiones::proyeccion))
        .route(
            "/inversiones/:id/rendimientos",
            get(rendimientos::listar).post(rendimientos::registrar),
        )
}
