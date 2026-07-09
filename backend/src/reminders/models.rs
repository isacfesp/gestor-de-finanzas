// =====================================================================
// models.rs — Structs de datos del módulo reminders.
// =====================================================================

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Refleja una fila de `notifications`. `reference_id` apunta al
/// recurso que originó la alerta (suscripción o presupuesto).
#[derive(Debug, Serialize)]
pub struct Notificacion {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub title: String,
    pub body: String,
    pub reference_id: Option<Uuid>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosNotificaciones {
    pub leidas: Option<bool>,
}
