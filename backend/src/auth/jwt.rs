// =====================================================================
// jwt.rs — Emisión y validación de access tokens (JWT).
//
// El access token es de vida corta (15 min): si alguien lo roba,
// le sirve poco tiempo. Para sesiones largas está el refresh token
// (ver tokens.rs), que sí se puede revocar.
// =====================================================================

use chrono::Utc;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errores::AppError;

/// Minutos de vida del access token.
pub const DURACION_ACCESS_MIN: i64 = 15;

/// Contenido (claims) que viaja firmado dentro del JWT.
///
/// El token NO está cifrado, solo firmado: cualquiera puede leerlo,
/// pero nadie puede modificarlo sin conocer JWT_SECRET. Por eso aquí
/// nunca se mete información sensible.
#[derive(Serialize, Deserialize)]
pub struct Claims {
    /// "subject": el id del usuario dueño del token.
    pub sub: Uuid,
    /// Rol global: "dev" o "usuario".
    pub rol: String,
    /// Fecha de expiración (segundos unix). jsonwebtoken la valida solo.
    pub exp: i64,
    /// Fecha de emisión (segundos unix).
    pub iat: i64,
}

/// Lee el secreto de firma desde la variable de entorno JWT_SECRET.
fn secreto() -> Result<String, AppError> {
    std::env::var("JWT_SECRET")
        .map_err(|_| AppError::Interno("JWT_SECRET no está definida en el .env".to_string()))
}

/// Crea un access token firmado para el usuario dado.
pub fn emitir_access_token(user_id: Uuid, rol: &str) -> Result<String, AppError> {
    let ahora = Utc::now();
    let claims = Claims {
        sub: user_id,
        rol: rol.to_string(),
        iat: ahora.timestamp(),
        exp: (ahora + chrono::Duration::minutes(DURACION_ACCESS_MIN)).timestamp(),
    };
    encode(
        &Header::default(), // HS256 por defecto
        &claims,
        &EncodingKey::from_secret(secreto()?.as_bytes()),
    )
    .map_err(|e| AppError::Interno(format!("No se pudo firmar el JWT: {e}")))
}

/// Verifica la firma y la expiración de un access token.
///
/// Devuelve los claims si el token es válido; 401 en cualquier otro
/// caso (firma inválida, expirado, malformado).
pub fn validar_access_token(token: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secreto()?.as_bytes()),
        &Validation::default(),
    )
    .map(|datos| datos.claims)
    .map_err(|_| AppError::NoAutorizado("Token inválido o expirado".to_string()))
}
