// Puerta de entrada del módulo de administración (solo rol dev).
mod handlers;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

/// Sub-router montado en /admin (ver main.rs). Cada handler exige
/// SoloDev en su firma, así que ninguna ruta es accesible sin rol dev.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/usuarios", post(handlers::crear_usuario))
        .route(
            "/workspaces",
            post(handlers::crear_workspace).get(handlers::listar_workspaces),
        )
        .route("/workspaces/:id/miembros", post(handlers::asignar_miembro))
        .route("/invitaciones", post(handlers::crear_invitacion))
        .route("/auditoria", get(handlers::listar_auditoria))
}
