// Puerta de entrada del módulo accounting: transacciones, categorías,
// suscripciones y presupuestos. Todas las rutas exigen autenticación y
// verifican membresía al workspace de la URL (ver autorizacion.rs).
// pub(crate) porque goals y planned_transactions reutilizan
// validar_categoria() para no duplicar la regla de "categoría visible
// desde el workspace y del tipo esperado".
pub(crate) mod categorias;
mod presupuestos;
pub(crate) mod suscripciones;
mod transacciones;

pub mod models;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route(
            "/categorias",
            get(categorias::listar).post(categorias::crear),
        )
        .route("/categorias/:id", delete(categorias::eliminar))
        .route(
            "/transacciones",
            get(transacciones::listar).post(transacciones::crear),
        )
        .route(
            "/transacciones/:id",
            get(transacciones::obtener)
                .put(transacciones::actualizar)
                .delete(transacciones::eliminar),
        )
        .route(
            "/suscripciones",
            get(suscripciones::listar).post(suscripciones::crear),
        )
        .route(
            "/suscripciones/proximos-cobros",
            get(suscripciones::proximos_cobros),
        )
        .route(
            "/suscripciones/:id",
            put(suscripciones::actualizar).delete(suscripciones::eliminar),
        )
        .route(
            "/suscripciones/:id/marcar-cobrada",
            post(suscripciones::marcar_cobrada),
        )
        .route(
            "/presupuestos",
            get(presupuestos::listar).post(presupuestos::crear),
        )
        .route("/presupuestos/estado", get(presupuestos::estado))
        .route("/presupuestos/:id", delete(presupuestos::eliminar))
}
