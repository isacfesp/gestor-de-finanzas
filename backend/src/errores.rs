// =====================================================================
// errores.rs — Tipo de error único para toda la aplicación.
//
// En vez de que cada handler arme tuplas (StatusCode, Json) a mano,
// los handlers devuelven Result<_, AppError> y usan el operador `?`.
// Axum convierte el AppError en una respuesta HTTP gracias al
// trait IntoResponse implementado abajo.
// =====================================================================

use axum::{Json, http::StatusCode, response::IntoResponse, response::Response};
use serde_json::json;

/// Errores que un handler puede devolver al cliente.
///
/// Cada variante se traduce a un código HTTP concreto. Las variantes
/// con String llevan el mensaje que verá el cliente — cuidado con no
/// incluir información sensible en ellos.
#[derive(Debug)]
pub enum AppError {
    /// 401 — no autenticado o credenciales/token inválidos.
    NoAutorizado(String),
    /// 403 — autenticado, pero sin permiso para esta operación.
    Prohibido(String),
    /// 404 — el recurso no existe (o no es visible para este usuario).
    NoEncontrado(String),
    /// 409 — conflicto con el estado actual (p. ej. email duplicado).
    Conflicto(String),
    /// 422 — los datos recibidos no pasan las validaciones.
    NoProcesable(String),
    /// 429 — demasiados intentos; la cuenta está bloqueada temporalmente.
    DemasiadosIntentos,
    /// 500 — error interno. El detalle se loggea en el servidor pero
    /// NUNCA se envía al cliente (podría revelar estructura interna).
    Interno(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (codigo, mensaje) = match self {
            AppError::NoAutorizado(m) => (StatusCode::UNAUTHORIZED, m),
            AppError::Prohibido(m) => (StatusCode::FORBIDDEN, m),
            AppError::NoEncontrado(m) => (StatusCode::NOT_FOUND, m),
            AppError::Conflicto(m) => (StatusCode::CONFLICT, m),
            AppError::NoProcesable(m) => (StatusCode::UNPROCESSABLE_ENTITY, m),
            AppError::DemasiadosIntentos => (
                StatusCode::TOO_MANY_REQUESTS,
                "Demasiados intentos, espera unos minutos".to_string(),
            ),
            AppError::Interno(detalle) => {
                // El detalle real solo queda en los logs del servidor.
                eprintln!("ERROR INTERNO: {detalle}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Error interno del servidor".to_string(),
                )
            }
        };
        (codigo, Json(json!({ "error": mensaje }))).into_response()
    }
}

// Conversión automática: cualquier error de SQLx que se propague con `?`
// se vuelve un 500 genérico (el detalle queda en los logs).
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Interno(format!("sqlx: {e}"))
    }
}

// Ídem para errores de bcrypt (hash/verificación de contraseñas).
impl From<bcrypt::BcryptError> for AppError {
    fn from(e: bcrypt::BcryptError) -> Self {
        AppError::Interno(format!("bcrypt: {e}"))
    }
}
