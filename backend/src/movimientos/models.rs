// =====================================================================
// models.rs — Structs de datos del módulo movimientos (lectura de
// audit_log con alcance de workspace).
// =====================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Una fila de la bitácora, ya resuelto el nombre de quién la generó.
#[derive(Debug, Serialize)]
pub struct Movimiento {
    pub id: Uuid,
    pub actor_name: String,
    pub action: String,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PaginacionMovimientos {
    pub limite: Option<i64>,
    pub desplazamiento: Option<i64>,
}
