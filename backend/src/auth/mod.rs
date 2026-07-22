// Puerta de entrada del módulo de autenticación.
//
// Expone el sub-router de /auth y las piezas que otros módulos
// necesitan: extractores (UsuarioAutenticado, SoloDev), la verificación
// de membresía por workspace y las utilidades de tokens.
pub mod autorizacion;
pub mod extractores;
pub mod jwt;
pub mod tokens;

mod handlers;

use axum::{
    Router,
    routing::{get, post},
};
use sqlx::PgPool;

/// Sub-router montado en /auth (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/login", post(handlers::login))
        .route("/refresh", post(handlers::refresh))
        .route("/logout", post(handlers::logout))
        .route("/yo", get(handlers::yo))
        .route("/mis-workspaces", get(handlers::mis_workspaces))
        .route("/invitaciones/aceptar", post(handlers::aceptar_invitacion))
        .route(
            "/solicitar-recuperacion",
            post(handlers::solicitar_recuperacion),
        )
        .route("/recuperar-password", post(handlers::recuperar_password))
}
