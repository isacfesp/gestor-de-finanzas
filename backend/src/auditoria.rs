// =====================================================================
// auditoria.rs — Bitácora de acciones sensibles (tabla audit_log).
//
// La auditoría es "best effort": si el INSERT falla, se loggea el
// problema pero NO se interrumpe la operación principal — un fallo
// al auditar no debe tirar un login válido.
// =====================================================================

use sqlx::PgPool;
use uuid::Uuid;

/// Acciones que se registran. Usar siempre estas constantes (y no
/// strings sueltos) para poder filtrar la bitácora sin errores de dedo.
pub mod acciones {
    pub const LOGIN_OK: &str = "login_ok";
    pub const LOGIN_FALLIDO: &str = "login_fallido";
    pub const LOGIN_BLOQUEADO: &str = "login_bloqueado";
    pub const LOGOUT: &str = "logout";
    pub const REFRESH_REUSO: &str = "refresh_reuso";
    pub const USUARIO_CREADO: &str = "usuario_creado";
    pub const WORKSPACE_CREADO: &str = "workspace_creado";
    pub const MIEMBRO_ASIGNADO: &str = "miembro_asignado";
    pub const INVITACION_CREADA: &str = "invitacion_creada";
    pub const INVITACION_ACEPTADA: &str = "invitacion_aceptada";
    pub const BOOTSTRAP_DEV: &str = "bootstrap_dev";
}

/// Registra una acción en la bitácora.
///
/// `user_id` es None cuando no hay usuario identificado (p. ej. un
/// intento de login con un email inexistente). `detalle` es JSON libre
/// con contexto útil — nunca incluir contraseñas ni tokens.
pub async fn registrar(
    pool: &PgPool,
    user_id: Option<Uuid>,
    accion: &str,
    detalle: serde_json::Value,
) {
    let resultado = sqlx::query!(
        "INSERT INTO audit_log (user_id, action, detail) VALUES ($1, $2, $3)",
        user_id,
        accion,
        detalle
    )
    .execute(pool)
    .await;

    if let Err(e) = resultado {
        eprintln!("AVISO: no se pudo registrar auditoría '{accion}': {e}");
    }
}
