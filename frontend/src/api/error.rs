//! Error uniforme para cualquier llamada a la API del backend.

use serde::Deserialize;

/// Error que puede devolver cualquier función de `api::*`.
#[derive(Debug, Clone)]
pub enum ApiError {
    /// El backend respondió (con un código 4xx/5xx) y trae un mensaje
    /// legible en el cuerpo `{"error": "..."}` que arma `AppError`.
    Servidor(String),
    /// La petición no llegó a completarse: red caída, JSON corrupto,
    /// CORS, etc. El mensaje viene de `gloo_net` y es técnico, no se
    /// debe mostrar tal cual al usuario final.
    Red(String),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::Servidor(mensaje) => write!(f, "{mensaje}"),
            ApiError::Red(detalle) => write!(f, "No se pudo conectar con el servidor ({detalle})"),
        }
    }
}

/// Forma exacta del cuerpo de error que devuelve `AppError` en el
/// backend (ver `backend/src/errores.rs`).
#[derive(Deserialize)]
pub(crate) struct ErrorCuerpo {
    pub error: String,
}
