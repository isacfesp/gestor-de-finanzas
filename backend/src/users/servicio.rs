// =====================================================================
// servicio.rs — Lógica interna de usuarios (sin rutas HTTP).
//
// No hay registro público en este sistema: las cuentas las crea el
// dev (módulo admin) o se crean al aceptar una invitación (módulo
// auth). Ambos caminos reutilizan crear_usuario() de este archivo.
// =====================================================================

use sqlx::PgConnection;

use crate::errores::AppError;
use crate::users::models::User;

/// Reglas de contraseña: mínimo 12 caracteres (frase antes que símbolos
/// raros) y máximo 72 bytes, que es el límite real que bcrypt procesa —
/// más allá de 72 bytes, bcrypt IGNORA el resto en silencio.
pub fn validar_password(password: &str) -> Result<(), AppError> {
    if password.chars().count() < 12 {
        return Err(AppError::NoProcesable(
            "La contraseña debe tener al menos 12 caracteres".to_string(),
        ));
    }
    if password.len() > 72 {
        return Err(AppError::NoProcesable(
            "La contraseña no puede superar 72 bytes".to_string(),
        ));
    }
    Ok(())
}

/// Normaliza el email para que "Ana@Mail.com " y "ana@mail.com" sean la
/// misma cuenta: sin espacios alrededor y en minúsculas.
pub fn normalizar_email(email: &str) -> Result<String, AppError> {
    let limpio = email.trim().to_lowercase();
    // Validación mínima de formato; la verdadera prueba es que el
    // correo reciba la invitación.
    if limpio.len() < 5 || !limpio.contains('@') || limpio.contains(' ') {
        return Err(AppError::NoProcesable("Email inválido".to_string()));
    }
    Ok(limpio)
}

/// Crea un usuario con la contraseña ya validada y hasheada con bcrypt.
///
/// Recibe una conexión (&mut PgConnection) en lugar del pool para poder
/// ejecutarse DENTRO de una transacción del que llama: si el paso
/// siguiente falla (p. ej. asignar el workspace), el usuario tampoco
/// queda creado.
///
/// Falla con 409 si el email ya está registrado.
pub async fn crear_usuario(
    conexion: &mut PgConnection,
    name: &str,
    email: &str,
    password: &str,
    role: &str,
) -> Result<User, AppError> {
    let email = normalizar_email(email)?;
    validar_password(password)?;

    if name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;

    let resultado = sqlx::query_as!(
        User,
        "INSERT INTO users (name, email, password_hash, role)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, email, password_hash, role, is_active,
                   failed_login_attempts, locked_until, created_at",
        name.trim(),
        email,
        hash,
        role
    )
    .fetch_one(conexion)
    .await;

    match resultado {
        Ok(usuario) => Ok(usuario),
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("users_email_unique") => Err(
            AppError::Conflicto("El email ya está registrado".to_string()),
        ),
        Err(e) => Err(e.into()),
    }
}

/// Busca un usuario por email normalizado. None si no existe.
pub async fn buscar_por_email(
    conexion: &mut PgConnection,
    email: &str,
) -> Result<Option<User>, AppError> {
    let email = normalizar_email(email)?;
    let fila = sqlx::query_as!(
        User,
        "SELECT id, name, email, password_hash, role, is_active,
                failed_login_attempts, locked_until, created_at
         FROM users WHERE email = $1",
        email
    )
    .fetch_optional(conexion)
    .await?;
    Ok(fila)
}
