//! Llamadas a `/admin/*` (backend `admin`, todo protegido `SoloDev`).
//! Alimenta el panel de administración (`pages::admin`) y, con
//! `listar_workspaces`, también el atajo interino de `crate::workspace`
//! para el rol `dev` (ver ese módulo).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

// ------------------------------- Workspaces -------------------------------

/// Workspace tal como lo devuelve `GET /admin/workspaces`.
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub miembros: i64,
}

/// GET /admin/workspaces
pub async fn listar_workspaces(token: &str) -> Result<Vec<Workspace>, ApiError> {
    client::get("/admin/workspaces", token).await
}

#[derive(Debug, Serialize)]
pub struct CrearWorkspaceDatos<'a> {
    pub name: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceCreado {
    pub id: Uuid,
    pub name: String,
}

/// POST /admin/workspaces
pub async fn crear_workspace(
    datos: &CrearWorkspaceDatos<'_>,
    token: &str,
) -> Result<WorkspaceCreado, ApiError> {
    client::post("/admin/workspaces", datos, token).await
}

/// DELETE /admin/workspaces/:id — borra el tenant y TODA su
/// información (transacciones, cuentas, metas, inversiones,
/// membresías, etc.), en cascada. Irreversible.
pub async fn eliminar_workspace(workspace_id: Uuid, token: &str) -> Result<(), ApiError> {
    client::delete(&format!("/admin/workspaces/{workspace_id}"), token).await
}

// --------------------------------- Usuarios --------------------------------

/// Usuario tal como lo devuelve el backend en las respuestas de
/// `/admin/usuarios` — distinto de `api::auth::Usuario` (que es lo
/// mínimo que necesita la sesión), porque aquí sí hace falta
/// `is_active` para decidir si mostrar "Desactivar" o "Reactivar".
#[derive(Debug, Clone, Deserialize)]
pub struct UsuarioAdmin {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CrearUsuarioDatos<'a> {
    pub name: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}

/// POST /admin/usuarios
pub async fn crear_usuario(
    datos: &CrearUsuarioDatos<'_>,
    token: &str,
) -> Result<UsuarioAdmin, ApiError> {
    client::post("/admin/usuarios", datos, token).await
}

/// GET /admin/usuarios
pub async fn listar_usuarios(token: &str) -> Result<Vec<UsuarioAdmin>, ApiError> {
    client::get("/admin/usuarios", token).await
}

/// POST /admin/usuarios/:id/desactivar
pub async fn desactivar_usuario(id: Uuid, token: &str) -> Result<UsuarioAdmin, ApiError> {
    client::post_vacio(&format!("/admin/usuarios/{id}/desactivar"), token).await
}

/// POST /admin/usuarios/:id/reactivar
pub async fn reactivar_usuario(id: Uuid, token: &str) -> Result<UsuarioAdmin, ApiError> {
    client::post_vacio(&format!("/admin/usuarios/{id}/reactivar"), token).await
}

// ------------------------------- Membresías --------------------------------

#[derive(Debug, Serialize)]
pub struct AsignarMiembroDatos<'a> {
    pub email: &'a str,
    pub role: &'a str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MiembroAsignado {
    pub mensaje: String,
    pub user_id: Uuid,
}

/// POST /admin/workspaces/:id/miembros
pub async fn asignar_miembro(
    workspace_id: Uuid,
    datos: &AsignarMiembroDatos<'_>,
    token: &str,
) -> Result<MiembroAsignado, ApiError> {
    client::post(
        &format!("/admin/workspaces/{workspace_id}/miembros"),
        datos,
        token,
    )
    .await
}

#[derive(Debug, Clone, Deserialize)]
pub struct MiembroWorkspace {
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub joined_at: DateTime<Utc>,
}

/// GET /admin/workspaces/:id/miembros
pub async fn listar_miembros(
    workspace_id: Uuid,
    token: &str,
) -> Result<Vec<MiembroWorkspace>, ApiError> {
    client::get(&format!("/admin/workspaces/{workspace_id}/miembros"), token).await
}

/// DELETE /admin/workspaces/:id/miembros/:user_id
pub async fn eliminar_miembro(
    workspace_id: Uuid,
    user_id: Uuid,
    token: &str,
) -> Result<(), ApiError> {
    client::delete(
        &format!("/admin/workspaces/{workspace_id}/miembros/{user_id}"),
        token,
    )
    .await
}

// ------------------------------ Invitaciones -------------------------------

#[derive(Debug, Serialize)]
pub struct CrearInvitacionDatos<'a> {
    pub workspace_id: Uuid,
    pub email: &'a str,
    pub role: &'a str,
}

/// El `token` solo se muestra una vez — el backend lo guarda hasheado,
/// no hay forma de volver a pedirlo después.
#[derive(Debug, Clone, Deserialize)]
pub struct InvitacionCreada {
    pub mensaje: String,
    pub token: String,
    pub expira: DateTime<Utc>,
}

/// POST /admin/invitaciones
pub async fn crear_invitacion(
    datos: &CrearInvitacionDatos<'_>,
    token: &str,
) -> Result<InvitacionCreada, ApiError> {
    client::post("/admin/invitaciones", datos, token).await
}

// -------------------------------- Auditoría ---------------------------------

/// A diferencia de `api::movimientos::Movimiento` (ya scoped a un
/// workspace), esta es la vista global — cruza tenants, por eso trae
/// también `workspace_name`.
#[derive(Debug, Clone, Deserialize)]
pub struct EntradaAuditoria {
    pub actor_name: String,
    pub workspace_name: Option<String>,
    pub action: String,
    pub detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// GET /admin/auditoria?limite=&desplazamiento=
pub async fn listar_auditoria(
    limite: Option<i64>,
    desplazamiento: Option<i64>,
    token: &str,
) -> Result<Vec<EntradaAuditoria>, ApiError> {
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
    client::get(&format!("/admin/auditoria{query}"), token).await
}
