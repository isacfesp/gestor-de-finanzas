// Puerta de entrada del módulo accounts: cuentas/billeteras y las
// transferencias de dinero entre ellas. Todas las rutas exigen
// autenticación y verifican membresía al workspace de la URL.
mod cuentas;
mod transferencias;

pub mod models;

pub(crate) use cuentas::validar_cuenta;

use axum::{
    Router,
    routing::{get, put},
};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/cuentas", get(cuentas::listar).post(cuentas::crear))
        .route(
            "/cuentas/:id",
            put(cuentas::actualizar).delete(cuentas::eliminar),
        )
        .route(
            "/transferencias",
            get(transferencias::listar).post(transferencias::crear),
        )
}
