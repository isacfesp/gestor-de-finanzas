//! Llamadas a `/admin/*`. Hoy solo se usa `listar_workspaces` como parte
//! del atajo interino de `crate::workspace` (ver ese módulo) — el resto
//! de endpoints de `/admin` (alta de usuarios, invitaciones, auditoría)
//! no tiene pantalla propia todavía.

use serde::Deserialize;
use uuid::Uuid;

use super::client;
use super::error::ApiError;

/// Workspace tal como lo devuelve `GET /admin/workspaces`. El backend
/// también manda `created_at` y `miembros`, pero no se piden porque
/// nadie los usa todavía — serde ignora los campos no declarados.
#[derive(Debug, Clone, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
}

/// GET /admin/workspaces — solo dev. La usa `crate::workspace` para
/// resolver el workspace activo mientras no exista un endpoint de
/// autoservicio ("mis workspaces") para el rol `usuario`.
pub async fn listar_workspaces(access_token: &str) -> Result<Vec<Workspace>, ApiError> {
    client::get("/admin/workspaces", access_token).await
}
