//! Workspace activo de la sesión.
//!
//! Se resuelve con `GET /auth/mis-workspaces`. Si el usuario tiene
//! acceso a más de uno (hoy solo pasa con `dev`, pero nada lo impide
//! para un usuario normal invitado a varios tenants), se recuerda cuál
//! eligió la última vez en localStorage; si no hay preferencia guardada
//! (o ya no es válida) se usa el primero de la lista.

use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use uuid::Uuid;

use crate::api;
use crate::auth::{AuthContext, token_vigente};

/// Clave de localStorage donde se recuerda el workspace elegido. Vive
/// aparte de `gestor.sesion` (auth) porque cambiar de workspace no
/// debe tocar la sesión, y cerrar sesión sí debe olvidar esta elección
/// (ver `crate::auth::AuthContext::limpiar`).
pub const CLAVE_WORKSPACE: &str = "gestor.workspace_activo";

#[derive(Debug, Clone)]
enum Estado {
    Cargando,
    Listo {
        lista: Vec<api::auth::WorkspaceResumen>,
        activo_id: Uuid,
    },
    Error(String),
}

/// Handle liviano y `Copy` al workspace activo. Se obtiene con
/// `use_workspace()` desde cualquier componente protegido.
#[derive(Copy, Clone)]
pub struct WorkspaceContext(RwSignal<Estado>);

impl WorkspaceContext {
    /// Id del workspace activo, si ya se resolvió.
    pub fn id(&self) -> Option<Uuid> {
        match self.0.get() {
            Estado::Listo { activo_id, .. } => Some(activo_id),
            _ => None,
        }
    }

    pub fn nombre(&self) -> Option<String> {
        match self.0.get() {
            Estado::Listo { lista, activo_id } => lista
                .into_iter()
                .find(|w| w.id == activo_id)
                .map(|w| w.name),
            _ => None,
        }
    }

    /// Todos los workspaces a los que el usuario tiene acceso. Vacío
    /// si todavía está cargando o hubo error. El selector del header
    /// (`components::shell`) solo se muestra cuando hay más de uno.
    pub fn lista(&self) -> Vec<api::auth::WorkspaceResumen> {
        match self.0.get() {
            Estado::Listo { lista, .. } => lista,
            _ => Vec::new(),
        }
    }

    /// `true` si el usuario es admin o dev en el workspace activo — así
    /// deciden las páginas si mostrar cuentas ajenas (solo lectura) o
    /// el selector de alcance de métricas de otros miembros.
    pub fn puede_supervisar(&self) -> bool {
        match self.0.get() {
            Estado::Listo { lista, activo_id } => lista
                .into_iter()
                .find(|w| w.id == activo_id)
                .map(|w| w.role == "admin" || w.role == "dev")
                .unwrap_or(false),
            _ => false,
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

    /// Cambia el workspace activo y recarga la página completa. Se
    /// recarga en vez de solo actualizar la señal porque hoy la
    /// mayoría de las páginas leen `workspace.id()` una sola vez al
    /// montar (no como dependencia reactiva de sus recursos), así que
    /// una recarga es el único cambio de bajo riesgo que garantiza que
    /// todo se vuelva a pedir con el `workspace_id` nuevo.
    pub fn cambiar(&self, id: Uuid) {
        let _ = LocalStorage::set(CLAVE_WORKSPACE, id.to_string());
        if let Some(window) = web_sys::window() {
            let _ = window.location().reload();
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
        Ok(lista) => {
            if lista.is_empty() {
                workspace.0.set(Estado::Error(
                    "Todavía no hay ningún workspace asignado".to_string(),
                ));
                return;
            }

            let preferido: Option<Uuid> = LocalStorage::get::<String>(CLAVE_WORKSPACE)
                .ok()
                .and_then(|id| id.parse().ok());

            let activo_id = preferido
                .filter(|id| lista.iter().any(|w| w.id == *id))
                .unwrap_or(lista[0].id);

            // Si no había preferencia (o ya no era válida), se guarda
            // la elegida como nuevo default para la próxima carga.
            let _ = LocalStorage::set(CLAVE_WORKSPACE, activo_id.to_string());

            workspace.0.set(Estado::Listo { lista, activo_id });
        }
        Err(error) => workspace.0.set(Estado::Error(error.to_string())),
    }
}
