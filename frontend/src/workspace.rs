//! Workspace activo de la sesión.
//!
//! Se resuelve con `GET /auth/mis-workspaces` y se toma el primero
//! como activo. Ese endpoint ya distingue el rol del usuario (un dev
//! ve todos los tenants, un usuario normal solo los suyos) — antes se
//! usaba `GET /admin/workspaces` (solo `dev`) como atajo interino,
//! documentado como pendiente en `CLAUDE.md`; ya está resuelto.

use leptos::prelude::*;

use crate::api;
use crate::auth::{AuthContext, token_vigente};

#[derive(Debug, Clone)]
enum Estado {
    Cargando,
    Listo(api::auth::WorkspaceResumen),
    Error(String),
}

/// Handle liviano y `Copy` al workspace activo. Se obtiene con
/// `use_workspace()` desde cualquier componente protegido.
#[derive(Copy, Clone)]
pub struct WorkspaceContext(RwSignal<Estado>);

impl WorkspaceContext {
    /// Id del workspace activo, si ya se resolvió.
    pub fn id(&self) -> Option<uuid::Uuid> {
        match self.0.get() {
            Estado::Listo(ws) => Some(ws.id),
            _ => None,
        }
    }

    pub fn nombre(&self) -> Option<String> {
        match self.0.get() {
            Estado::Listo(ws) => Some(ws.name),
            _ => None,
        }
    }

    /// Mensaje de error si no se pudo resolver (sin workspaces, sesión
    /// vencida, etc.), para mostrarlo en vez de dejar la pantalla en
    /// blanco.
    pub fn error(&self) -> Option<String> {
        match self.0.get() {
            Estado::Error(mensaje) => Some(mensaje),
            _ => None,
        }
    }
}

/// Deja el contexto disponible (en estado "cargando") para toda la app.
/// Se llama una sola vez, en la raíz de `App`, junto a
/// `provide_auth_context()`.
pub fn provide_workspace_context() -> WorkspaceContext {
    let contexto = WorkspaceContext(RwSignal::new(Estado::Cargando));
    provide_context(contexto);
    contexto
}

pub fn use_workspace() -> WorkspaceContext {
    use_context::<WorkspaceContext>()
        .expect("WorkspaceContext no está disponible: falta provide_workspace_context() en App")
}

/// Resuelve el workspace activo contra el backend y guarda el
/// resultado. La llama `ProtectedShell` una vez, en cuanto detecta
/// sesión iniciada.
pub async fn cargar_activo(auth: AuthContext, workspace: WorkspaceContext) {
    let Some(token) = token_vigente(auth).await else {
        workspace
            .0
            .set(Estado::Error("No hay sesión activa".to_string()));
        return;
    };

    match api::auth::mis_workspaces(&token).await {
        Ok(lista) => match lista.into_iter().next() {
            Some(primero) => workspace.0.set(Estado::Listo(primero)),
            None => workspace.0.set(Estado::Error(
                "Todavía no hay ningún workspace asignado".to_string(),
            )),
        },
        Err(error) => workspace.0.set(Estado::Error(error.to_string())),
    }
}
