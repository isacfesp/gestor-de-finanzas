//! Estado de sesión de la aplicación: quién está autenticado y con
//! qué tokens. Vive en un `RwSignal` provisto por contexto desde la
//! raíz de `App`, así cualquier página o componente puede leerlo con
//! `use_auth()` sin pasarlo a mano por cada nivel.

use gloo_storage::{LocalStorage, Storage};
use leptos::prelude::*;
use serde::{Deserialize, Serialize};

use crate::api;

const CLAVE_STORAGE: &str = "gestor.sesion";

/// Todo lo que la app necesita recordar entre recargas de página.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sesion {
    pub access_token: String,
    pub refresh_token: String,
    /// Milisegundos desde epoch (`Date.now()`) en que expira el access
    /// token. Se calcula una sola vez al iniciar sesión, a partir de
    /// `expires_in`, para no tener que decodificar el JWT en el cliente.
    pub expira_en_ms: f64,
    pub usuario: api::auth::Usuario,
}

/// Handle liviano y `Copy` al estado de sesión. Se obtiene con
/// `use_auth()` desde cualquier componente.
#[derive(Copy, Clone)]
pub struct AuthContext {
    sesion: RwSignal<Option<Sesion>>,
}

impl AuthContext {
    pub fn is_logged_in(&self) -> bool {
        self.sesion.with(Option::is_some)
    }

    pub fn usuario(&self) -> Option<api::auth::Usuario> {
        self.sesion.get().map(|s| s.usuario)
    }

    /// Rol global `dev` — mismo nombre que usa el backend
    /// (`UsuarioAutenticado::es_dev`), para no repetir la comparación
    /// de string en cada lugar que necesite filtrar por rol (nav,
    /// guard de la página Admin).
    pub fn es_dev(&self) -> bool {
        self.usuario().map(|u| u.role == "dev").unwrap_or(false)
    }

    fn establecer(&self, sesion: Sesion) {
        // Si guardar en localStorage falla (modo privado, cuota llena),
        // la sesión sigue funcionando en memoria para esta pestaña.
        let _ = LocalStorage::set(CLAVE_STORAGE, &sesion);
        self.sesion.set(Some(sesion));
    }

    fn limpiar(&self) {
        LocalStorage::delete(CLAVE_STORAGE);
        // También se olvida el workspace elegido: que un login distinto
        // en el mismo navegador no arranque con la preferencia de otra
        // cuenta (crate::workspace::cargar_activo la revalida igual,
        // esto es higiene, no una corrección de seguridad).
        LocalStorage::delete(crate::workspace::CLAVE_WORKSPACE);
        self.sesion.set(None);
    }
}

/// Crea el contexto de auth —leyendo una sesión guardada, si hay una
/// vigente en localStorage— y lo deja disponible para toda la app.
/// Se llama una sola vez, en la raíz de `App`.
pub fn provide_auth_context() -> AuthContext {
    let inicial = LocalStorage::get::<Sesion>(CLAVE_STORAGE).ok();
    let contexto = AuthContext {
        sesion: RwSignal::new(inicial),
    };
    provide_context(contexto);
    contexto
}

/// Atajo para leer el contexto desde cualquier componente hijo de `App`.
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>()
        .expect("AuthContext no está disponible: falta provide_auth_context() en App")
}

/// Inicia sesión contra el backend y, si sale bien, deja la sesión
/// guardada con los datos del usuario ya cargados (así el resto de la
/// app no necesita volver a pedirlos).
pub async fn iniciar_sesion(
    auth: AuthContext,
    email: &str,
    password: &str,
) -> Result<(), api::ApiError> {
    let tokens = api::auth::login(email, password).await?;
    let usuario = api::auth::yo(&tokens.access_token).await?;
    let expira_en_ms = js_sys::Date::now() + (tokens.expires_in as f64 * 1000.0);

    auth.establecer(Sesion {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expira_en_ms,
        usuario,
    });
    Ok(())
}

/// Cierra sesión: intenta avisar al backend para revocar el refresh
/// token y, pase lo que pase con esa llamada, limpia el estado local
/// — el usuario espera salir aunque el servidor no responda.
pub async fn cerrar_sesion(auth: AuthContext) {
    if let Some(sesion) = auth.sesion.get_untracked() {
        let _ = api::auth::logout(&sesion.refresh_token, &sesion.access_token).await;
    }
    auth.limpiar();
}

// Todavía no hay páginas de módulo con llamadas autenticadas propias
// (son placeholders), así que nadie la usa aún — es la pieza base que
// usará cada módulo nuevo (cuentas, metas, inversiones...) para
// obtener un token antes de llamar a su API.
#[allow(dead_code)]
/// Devuelve un access token vigente, refrescándolo antes si ya venció
/// (el access token dura 15 min). Todas las funciones de `api::*` que
/// requieran autenticación deben pasar por aquí en vez de leer el
/// token guardado directo, para que una llamada nunca falle solo
/// porque pasó el tiempo.
pub async fn token_vigente(auth: AuthContext) -> Option<String> {
    let sesion_actual = auth.sesion.get_untracked()?;

    // Margen de 10 segundos para no arrancar una petición con un token
    // que vence a mitad de camino.
    let vence_pronto = js_sys::Date::now() >= sesion_actual.expira_en_ms - 10_000.0;
    if !vence_pronto {
        return Some(sesion_actual.access_token);
    }

    match api::auth::refresh(&sesion_actual.refresh_token).await {
        Ok(tokens) => {
            let expira_en_ms = js_sys::Date::now() + (tokens.expires_in as f64 * 1000.0);
            let token_nuevo = tokens.access_token.clone();
            auth.establecer(Sesion {
                access_token: tokens.access_token,
                refresh_token: tokens.refresh_token,
                expira_en_ms,
                usuario: sesion_actual.usuario,
            });
            Some(token_nuevo)
        }
        Err(error) => {
            // El refresh token ya no sirve (venció o se detectó reuso):
            // no hay forma de recuperar la sesión, se cierra.
            log::warn!("No se pudo refrescar la sesión: {error}");
            auth.limpiar();
            None
        }
    }
}
