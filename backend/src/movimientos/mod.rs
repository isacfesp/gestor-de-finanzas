// Puerta de entrada del módulo movimientos: lectura de la bitácora
// (audit_log) con alcance de workspace — cualquier miembro puede verla,
// a diferencia de `GET /admin/auditoria` que es solo para el rol dev.
mod consulta;

pub mod models;

use axum::{Router, routing::get};
use sqlx::PgPool;

/// Sub-router montado en /workspaces/:workspace_id (ver main.rs).
pub fn router() -> Router<PgPool> {
    Router::new().route("/movimientos", get(consulta::listar))
}
