// =====================================================================
// autorizacion.rs — Regla central del multi-tenant.
//
// TODO endpoint que toque datos de un workspace debe llamar a
// verificar_membresia() con el workspace_id tomado de la RUTA
// (nunca del body: el body lo controla el cliente).
// =====================================================================

use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

/// Rol efectivo del usuario dentro de un workspace concreto.
// allow(dead_code): los consumidores de esta pieza son los módulos de
// datos (accounting, goals...) que aún no existen. Quitar cuando llegue
// el primero.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RolWorkspace {
    /// El rol dev pasa por encima de las membresías: ve todo.
    DevGlobal,
    /// admin del workspace: permisos totales dentro de él.
    Admin,
    /// member: lectura y escritura de datos.
    Member,
}

/// Verifica que el usuario pueda operar en el workspace dado.
///
/// - dev → acceso siempre (DevGlobal).
/// - usuario → debe existir su fila en workspace_members.
///
/// Devuelve 404 (no 403) cuando no hay membresía: así un usuario ajeno
/// no puede distinguir entre "ese workspace no existe" y "existe pero
/// no es tuyo" — no se filtra ni la existencia del recurso.
// allow(dead_code): ver nota en RolWorkspace.
#[allow(dead_code)]
pub async fn verificar_membresia(
    pool: &PgPool,
    usuario: &UsuarioAutenticado,
    workspace_id: Uuid,
) -> Result<RolWorkspace, AppError> {
    if usuario.es_dev() {
        return Ok(RolWorkspace::DevGlobal);
    }

    let fila = sqlx::query_scalar!(
        "SELECT role FROM workspace_members WHERE workspace_id = $1 AND user_id = $2",
        workspace_id,
        usuario.id
    )
    .fetch_optional(pool)
    .await?;

    match fila.as_deref() {
        Some("admin") => Ok(RolWorkspace::Admin),
        Some("member") => Ok(RolWorkspace::Member),
        Some(otro) => Err(AppError::Interno(format!(
            "Rol de workspace desconocido: {otro}"
        ))),
        None => Err(AppError::NoEncontrado(
            "Workspace no encontrado".to_string(),
        )),
    }
}
