// =====================================================================
// tokens.rs — Tokens opacos: refresh tokens e invitaciones.
//
// Un token "opaco" es una cadena aleatoria sin significado (a
// diferencia del JWT, que lleva datos). El valor plano solo lo ve el
// cliente; en la base se guarda únicamente su hash SHA-256. Así, si
// alguien roba un dump de la base, no tiene ningún token utilizable.
// =====================================================================

use chrono::{DateTime, Utc};
use rand::RngCore;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errores::AppError;

/// Días de vida de un refresh token.
pub const DURACION_REFRESH_DIAS: i64 = 30;
/// Horas de vida de un link de invitación.
pub const DURACION_INVITACION_HORAS: i64 = 72;
/// Horas de vida de un link de recuperación de contraseña — corto
/// porque es un flujo sensible (permite cambiar la contraseña).
pub const DURACION_RESET_HORAS: i64 = 1;

/// Genera un token aleatorio de 32 bytes en formato hexadecimal.
///
/// Se usa OsRng (aleatoriedad del sistema operativo), que es la fuente
/// criptográficamente segura — nunca usar rand::thread_rng para tokens
/// de seguridad sin verificar que sea un CSPRNG.
pub fn generar_token_opaco() -> String {
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

/// Hash SHA-256 de un token, en hexadecimal. Es lo único que se guarda en la DB.
pub fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

/// Fila de refresh_tokens que usamos para validar y rotar.
pub struct RefreshGuardado {
    pub id: Uuid,
    pub user_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

/// Crea un refresh token nuevo para el usuario y devuelve el valor PLANO
/// (la única vez que existe fuera de la memoria del cliente).
pub async fn crear_refresh_token(pool: &PgPool, user_id: Uuid) -> Result<String, AppError> {
    let token = generar_token_opaco();
    let expira = Utc::now() + chrono::Duration::days(DURACION_REFRESH_DIAS);

    sqlx::query!(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
        user_id,
        hash_token(&token),
        expira
    )
    .execute(pool)
    .await?;

    Ok(token)
}

/// Busca un refresh token por su valor plano. Devuelve None si no existe.
pub async fn buscar_refresh_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<RefreshGuardado>, AppError> {
    let fila = sqlx::query_as!(
        RefreshGuardado,
        "SELECT id, user_id, expires_at FROM refresh_tokens WHERE token_hash = $1",
        hash_token(token)
    )
    .fetch_optional(pool)
    .await?;
    Ok(fila)
}

/// Marca un refresh token como usado/revocado poniendo su expiración en
/// el pasado. La fila NO se borra: conservarla permite detectar si
/// alguien intenta reusar un token viejo (señal de robo).
pub async fn revocar_refresh_token(pool: &PgPool, id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE refresh_tokens SET expires_at = now() WHERE id = $1",
        id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Revoca TODOS los refresh tokens de un usuario (cierra todas sus
/// sesiones). Se usa cuando se detecta reuso de un token rotado.
pub async fn revocar_todos_los_refresh(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query!(
        "UPDATE refresh_tokens SET expires_at = now() WHERE user_id = $1 AND expires_at > now()",
        user_id
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Limpieza oportunista: borra tokens vencidos hace más de 60 días.
/// Se llama en cada login para que la tabla no crezca sin límite.
pub async fn purgar_refresh_vencidos(pool: &PgPool) -> Result<(), AppError> {
    sqlx::query!("DELETE FROM refresh_tokens WHERE expires_at < now() - interval '60 days'")
        .execute(pool)
        .await?;
    Ok(())
}
