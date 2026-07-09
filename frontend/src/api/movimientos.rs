//! Llamadas a `/workspaces/:workspace_id/movimientos` (backend
//! `movimientos`) — bitácora de actividad del workspace, distinta de
//! `/admin/auditoria` (esa es global y solo para el rol dev).

use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;

use super::client;
use super::error::ApiError;

#[derive(Debug, Clone, Deserialize)]
pub struct Movimiento {
    pub actor_name: String,
    pub action: String,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Traduce la constante cruda de `auditoria::acciones` (backend) a una
/// etiqueta legible en español. Si aparece una acción que no está en
/// esta lista (p. ej. una nueva del backend que no se sincronizó aquí
/// todavía), se muestra la acción cruda en vez de dejar la fila vacía.
pub fn etiqueta_accion(accion: &str) -> &str {
    match accion {
        // Globales — nunca aparecen en Movimientos (no tienen
        // workspace_id), solo en la auditoría global del panel de admin.
        "login_ok" => "Inició sesión",
        "login_fallido" => "Intento de inicio de sesión fallido",
        "login_bloqueado" => "Cuenta bloqueada por intentos fallidos",
        "logout" => "Cerró sesión",
        "refresh_reuso" => "Reuso de refresh token detectado",
        "usuario_creado" => "Creó un usuario",
        "usuario_desactivado" => "Desactivó un usuario",
        "usuario_reactivado" => "Reactivó un usuario",
        "bootstrap_dev" => "Arranque inicial del sistema",
        "workspace_creado" => "Creó el workspace",
        "miembro_asignado" => "Asignó un miembro",
        "miembro_eliminado" => "Eliminó un miembro",
        "invitacion_creada" => "Generó una invitación",
        "invitacion_aceptada" => "Aceptó una invitación",
        "cuenta_creada" => "Creó una cuenta",
        "cuenta_editada" => "Editó una cuenta",
        "cuenta_eliminada" => "Eliminó una cuenta",
        "transferencia_creada" => "Registró una transferencia",
        "etiqueta_creada" => "Creó una etiqueta",
        "etiqueta_eliminada" => "Eliminó una etiqueta",
        "etiqueta_asociada" => "Etiquetó una transacción",
        "etiqueta_desasociada" => "Quitó una etiqueta",
        "previsto_creado" => "Creó un previsto",
        "previsto_editado" => "Editó un previsto",
        "previsto_pagado" => "Marcó un previsto como pagado",
        "previsto_eliminado" => "Eliminó un previsto",
        "categoria_creada" => "Creó una categoría",
        "categoria_eliminada" => "Eliminó una categoría",
        "transaccion_creada" => "Registró una transacción",
        "transaccion_editada" => "Editó una transacción",
        "transaccion_eliminada" => "Eliminó una transacción",
        "suscripcion_creada" => "Creó una suscripción",
        "suscripcion_editada" => "Editó una suscripción",
        "suscripcion_cobrada" => "Marcó una suscripción como cobrada",
        "suscripcion_eliminada" => "Eliminó una suscripción",
        "presupuesto_guardado" => "Guardó un presupuesto",
        "presupuesto_eliminado" => "Eliminó un presupuesto",
        "meta_creada" => "Creó una meta",
        "meta_editada" => "Editó una meta",
        "meta_eliminada" => "Eliminó una meta",
        "aporte_registrado" => "Registró un aporte",
        "inversion_creada" => "Creó una inversión",
        "inversion_eliminada" => "Eliminó una inversión",
        "rendimiento_registrado" => "Registró un rendimiento",
        otra => otra,
    }
}

/// GET /workspaces/:workspace_id/movimientos?limite=&desplazamiento=
pub async fn listar_movimientos(
    workspace_id: Uuid,
    limite: Option<i64>,
    desplazamiento: Option<i64>,
    token: &str,
) -> Result<Vec<Movimiento>, ApiError> {
    let mut partes = Vec::new();
    if let Some(limite) = limite {
        partes.push(format!("limite={limite}"));
    }
    if let Some(desplazamiento) = desplazamiento {
        partes.push(format!("desplazamiento={desplazamiento}"));
    }
    let query = if partes.is_empty() {
        String::new()
    } else {
        format!("?{}", partes.join("&"))
    };
    client::get(
        &format!("/workspaces/{workspace_id}/movimientos{query}"),
        token,
    )
    .await
}
