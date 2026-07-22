//! Llamadas a `/auth/*`. Los structs reflejan exactamente lo que
//! espera y devuelve `backend/src/auth/handlers.rs`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

#[derive(Serialize)]
struct LoginDatos<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Serialize)]
struct RefreshDatos<'a> {
    refresh_token: &'a str,
}

/// Par de tokens que entrega el backend al iniciar sesión o refrescar.
/// El backend también manda `token_type` ("Bearer"), pero como nunca
/// cambia no se modela aquí — serde ignora los campos que no se piden.
#[derive(Debug, Clone, Deserialize)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Segundos de vida del access token.
    pub expires_in: i64,
}

/// Datos públicos del usuario autenticado (sin contraseña ni tokens).
/// Serialize además de Deserialize porque se guarda tal cual en
/// localStorage como parte de la sesión (ver `crate::auth::Sesion`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usuario {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
}

/// POST /auth/login
pub async fn login(email: &str, password: &str) -> Result<Tokens, ApiError> {
    client::post_publico("/auth/login", &LoginDatos { email, password }).await
}

// La usa crate::auth::token_vigente, que todavía no llama nadie porque
// ninguna página protegida hace llamadas autenticadas propias aún.
#[allow(dead_code)]
/// POST /auth/refresh — cambia un refresh token por un par nuevo (rotación).
pub async fn refresh(refresh_token: &str) -> Result<Tokens, ApiError> {
    client::post_publico("/auth/refresh", &RefreshDatos { refresh_token }).await
}

/// POST /auth/logout — revoca el refresh token actual (responde 204).
pub async fn logout(refresh_token: &str, access_token: &str) -> Result<(), ApiError> {
    client::post_sin_respuesta(
        "/auth/logout",
        &RefreshDatos { refresh_token },
        access_token,
    )
    .await
}

/// GET /auth/yo — datos del usuario autenticado.
pub async fn yo(access_token: &str) -> Result<Usuario, ApiError> {
    client::get("/auth/yo", access_token).await
}

/// Workspace visible para el usuario autenticado — lo mínimo que
/// necesita `crate::workspace` para resolver el activo, sin los campos
/// que solo le hacen falta al panel de administración
/// (`api::admin::Workspace` ya trae `created_at`/`miembros`).
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceResumen {
    pub id: Uuid,
    pub name: String,
    /// Rol efectivo del usuario en ESTE workspace: "admin", "member" o
    /// "dev" — decide si puede supervisar cuentas ajenas o inspeccionar
    /// métricas de otros miembros (ver `crate::workspace::puede_supervisar`).
    pub role: String,
}

/// GET /auth/mis-workspaces — cualquier usuario autenticado. Un dev ve
/// todos los tenants; un usuario normal solo los que tiene asignados.
/// Resuelve el pendiente anotado en `CLAUDE.md` sobre el selector de
/// workspace activo.
pub async fn mis_workspaces(access_token: &str) -> Result<Vec<WorkspaceResumen>, ApiError> {
    client::get("/auth/mis-workspaces", access_token).await
}

#[derive(Serialize)]
struct AceptarInvitacionDatos<'a> {
    token: &'a str,
    name: &'a str,
    password: &'a str,
}

/// POST /auth/invitaciones/aceptar — canjea el token de una invitación:
/// crea la cuenta (o la reutiliza si el email ya existía) y la une al
/// workspace. No devuelve tokens de sesión — el invitado debe iniciar
/// sesión aparte con `login`.
pub async fn aceptar_invitacion(token: &str, name: &str, password: &str) -> Result<(), ApiError> {
    client::post_publico::<_, serde_json::Value>(
        "/auth/invitaciones/aceptar",
        &AceptarInvitacionDatos {
            token,
            name,
            password,
        },
    )
    .await?;
    Ok(())
}

#[derive(Serialize)]
struct SolicitarRecuperacionDatos<'a> {
    email: &'a str,
}

/// POST /auth/solicitar-recuperacion — pide el link de recuperación de
/// contraseña. Responde 204 exista o no el email: nunca revela si una
/// cuenta está registrada.
pub async fn solicitar_recuperacion(email: &str) -> Result<(), ApiError> {
    client::post_publico_sin_respuesta(
        "/auth/solicitar-recuperacion",
        &SolicitarRecuperacionDatos { email },
    )
    .await
}

#[derive(Serialize)]
struct RecuperarPasswordDatos<'a> {
    token: &'a str,
    password: &'a str,
}

/// POST /auth/recuperar-password — canjea el token del correo y fija la
/// contraseña nueva. Cierra todas las sesiones activas del usuario.
pub async fn recuperar_password(token: &str, password: &str) -> Result<(), ApiError> {
    client::post_publico_sin_respuesta(
        "/auth/recuperar-password",
        &RecuperarPasswordDatos { token, password },
    )
    .await
}
