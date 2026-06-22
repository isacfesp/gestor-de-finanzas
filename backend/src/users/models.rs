// Define las estructuras de datos (Structs)
use serde::{Deserialize, Serialize};

// USUARIOS
#[derive(sqlx::FromRow)]
pub struct User {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct RegistroDatos {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RespuestaUsuario {
    pub id: uuid::Uuid,
    pub name: String,
    pub email: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
