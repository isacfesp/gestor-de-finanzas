// Puerta de entrada del módulo reminders: notificaciones (HTTP) y el
// motor en background que las genera. `motor` no forma parte del
// router — se lanza desde main.rs con tokio::spawn.
mod notificaciones;

pub mod models;
pub mod motor;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/notificaciones", get(notificaciones::listar))
        .route(
            "/notificaciones/:id/marcar-leida",
            post(notificaciones::marcar_leida),
        )
}
