// Puerta de entrada del módulo tags: etiquetas libres del workspace y
// su asociación muchos-a-muchos con transacciones.
mod asociaciones;
mod etiquetas;

pub mod models;

use axum::{
    Router,
    routing::{delete, get, post},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
///
/// Las rutas de asociación viven bajo /transacciones/:id/etiquetas
/// usando el mismo nombre de parámetro (:id) que el router de
/// accounting, porque axum exige que dos routers anidados en el mismo
/// prefijo usen nombres de parámetro idénticos en la misma posición.
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/etiquetas", get(etiquetas::listar).post(etiquetas::crear))
        .route("/etiquetas/:id", delete(etiquetas::eliminar))
        .route("/transacciones/:id/etiquetas", post(asociaciones::agregar))
        .route(
            "/transacciones/:id/etiquetas/:tag_id",
            delete(asociaciones::quitar),
        )
}
