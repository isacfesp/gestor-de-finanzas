// Define las estructuras de datos (structs) del dominio de usuarios.
use serde::Serialize;

// USUARIOS — refleja una fila completa de la tabla users.
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    /// Rol global: 'dev' o 'usuario'.
    pub role: String,
    /// false = cuenta desactivada, no puede iniciar sesión.
    pub is_active: bool,
    /// Logins fallidos consecutivos (se reinicia al entrar bien).
    pub failed_login_attempts: i32,
    /// Si tiene valor futuro, la cuenta está bloqueada hasta esa hora.
    pub locked_until: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Versión pública del usuario: lo que se devuelve en las respuestas.
/// Nunca incluye password_hash ni los campos de control interno.
#[derive(Serialize)]
pub struct RespuestaUsuario {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<User> for RespuestaUsuario {
    fn from(u: User) -> Self {
        RespuestaUsuario {
            id: u.id,
            name: u.name,
            email: u.email,
            role: u.role,
            is_active: u.is_active,
            created_at: u.created_at,
        }
    }
}
