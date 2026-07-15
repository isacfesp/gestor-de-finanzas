// =====================================================================
// handlers.rs — Endpoints de administración. TODOS exigen rol dev:
// el extractor SoloDev en la firma rechaza con 403 a cualquier otro.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auditoria::{self, acciones};
use crate::auth::extractores::SoloDev;
use crate::auth::tokens;
use crate::errores::AppError;
use crate::users::models::{RespuestaUsuario, User};
use crate::users::servicio;

/// Valida un rol de workspace (el CHECK de la tabla acepta solo estos).
fn validar_rol_workspace(rol: &str) -> Result<(), AppError> {
    if rol == "admin" || rol == "member" {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El rol debe ser 'admin' o 'member'".to_string(),
        ))
    }
}

/// Verifica que el workspace exista. 404 si no.
async fn workspace_existe(pool: &PgPool, workspace_id: Uuid) -> Result<(), AppError> {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM workspaces WHERE id = $1)",
        workspace_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false); // EXISTS nunca es NULL; el Option viene del macro de sqlx

    if existe {
        Ok(())
    } else {
        Err(AppError::NoEncontrado(
            "Workspace no encontrado".to_string(),
        ))
    }
}

// ------------------------- POST /admin/usuarios -------------------------

#[derive(Deserialize)]
pub struct CrearUsuarioDatos {
    pub name: String,
    pub email: String,
    /// Contraseña temporal que el dev comunica al familiar por un canal seguro.
    pub password: String,
}

/// Da de alta un usuario con rol global 'usuario'.
pub async fn crear_usuario(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Json(datos): Json<CrearUsuarioDatos>,
) -> Result<(StatusCode, Json<RespuestaUsuario>), AppError> {
    let mut tx = pool.begin().await?;
    let usuario = servicio::crear_usuario(
        &mut tx,
        &datos.name,
        &datos.email,
        &datos.password,
        "usuario",
    )
    .await?;
    tx.commit().await?;

    auditoria::registrar(
        &pool,
        None,
        Some(dev.id),
        acciones::USUARIO_CREADO,
        json!({"nuevo_usuario_id": usuario.id, "email": usuario.email}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(usuario.into())))
}

// ------------------------- GET /admin/usuarios -------------------------

/// Lista todos los usuarios del sistema — hace falta para poder elegir
/// a quién desactivar/reactivar desde el panel (esos endpoints piden el
/// `id` en la ruta, y hasta ahora no había forma de conocerlo).
pub async fn listar_usuarios(
    State(pool): State<PgPool>,
    SoloDev(_dev): SoloDev,
) -> Result<Json<Vec<RespuestaUsuario>>, AppError> {
    let filas = sqlx::query_as!(
        RespuestaUsuario,
        "SELECT id, name, email, role, is_active, created_at FROM users ORDER BY name"
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(filas))
}

// ------------------------- POST /admin/workspaces -------------------------

#[derive(Deserialize)]
pub struct CrearWorkspaceDatos {
    pub name: String,
}

#[derive(Serialize)]
pub struct RespuestaWorkspace {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<Utc>,
}

/// Crea un tenant (workspace). Solo dev puede — regla central del sistema.
pub async fn crear_workspace(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Json(datos): Json<CrearWorkspaceDatos>,
) -> Result<(StatusCode, Json<RespuestaWorkspace>), AppError> {
    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let fila = sqlx::query_as!(
        RespuestaWorkspace,
        "INSERT INTO workspaces (name, owner_id) VALUES ($1, $2)
         RETURNING id, name, created_at",
        datos.name.trim(),
        dev.id
    )
    .fetch_one(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(fila.id),
        Some(dev.id),
        acciones::WORKSPACE_CREADO,
        json!({"workspace_id": fila.id, "name": fila.name}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(fila)))
}

// ------------------------- GET /admin/workspaces -------------------------

#[derive(Serialize)]
pub struct WorkspaceListado {
    pub id: Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<Utc>,
    /// Cuántos usuarios tienen membresía en el workspace.
    pub miembros: i64,
}

/// Lista todos los tenants con su número de miembros (vista de auditoría).
pub async fn listar_workspaces(
    State(pool): State<PgPool>,
    SoloDev(_dev): SoloDev,
) -> Result<Json<Vec<WorkspaceListado>>, AppError> {
    let filas = sqlx::query_as!(
        WorkspaceListado,
        r#"SELECT w.id, w.name, w.created_at,
                  count(m.user_id) AS "miembros!"
           FROM workspaces w
           LEFT JOIN workspace_members m ON m.workspace_id = w.id
           GROUP BY w.id
           ORDER BY w.created_at"#
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(filas))
}

// ------------------------- POST /admin/workspaces/:id/miembros -------------------------

#[derive(Deserialize)]
pub struct AsignarMiembroDatos {
    pub email: String,
    pub role: String,
}

/// Asigna un usuario existente a un workspace (o actualiza su rol si ya
/// era miembro).
pub async fn asignar_miembro(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<AsignarMiembroDatos>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    validar_rol_workspace(&datos.role)?;
    workspace_existe(&pool, workspace_id).await?;

    let mut conexion = pool.acquire().await?;
    let usuario = servicio::buscar_por_email(&mut conexion, &datos.email)
        .await?
        .ok_or_else(|| AppError::NoEncontrado("Usuario no encontrado".to_string()))?;

    sqlx::query!(
        "INSERT INTO workspace_members (workspace_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (workspace_id, user_id) DO UPDATE SET role = $3",
        workspace_id,
        usuario.id,
        datos.role
    )
    .execute(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(dev.id),
        acciones::MIEMBRO_ASIGNADO,
        json!({"workspace_id": workspace_id, "user_id": usuario.id, "role": datos.role}),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(json!({"mensaje": "Miembro asignado", "user_id": usuario.id})),
    ))
}

// ------------------------- GET /admin/workspaces/:id/miembros -------------------------

#[derive(Serialize)]
pub struct MiembroWorkspace {
    pub user_id: Uuid,
    pub name: String,
    pub email: String,
    pub role: String,
    pub joined_at: chrono::DateTime<Utc>,
}

/// Lista los miembros de un workspace — hace falta para poder elegir a
/// quién eliminar desde el panel (`eliminar_miembro` pide el `user_id`
/// en la ruta, y hasta ahora no había forma de conocerlo).
pub async fn listar_miembros(
    State(pool): State<PgPool>,
    SoloDev(_dev): SoloDev,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<MiembroWorkspace>>, AppError> {
    workspace_existe(&pool, workspace_id).await?;

    let filas = sqlx::query_as!(
        MiembroWorkspace,
        r#"SELECT u.id AS user_id, u.name, u.email, m.role, m.joined_at
           FROM workspace_members m
           JOIN users u ON u.id = m.user_id
           WHERE m.workspace_id = $1
           ORDER BY u.name"#,
        workspace_id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

// ------------------------- POST /admin/invitaciones -------------------------

#[derive(Deserialize)]
pub struct CrearInvitacionDatos {
    pub workspace_id: Uuid,
    pub email: String,
    pub role: String,
}

/// Genera un link de invitación de un solo uso (expira en 72 h).
///
/// El token plano se devuelve UNA sola vez aquí; en la base queda solo
/// su hash. Si se pierde, se genera otra invitación.
pub async fn crear_invitacion(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Json(datos): Json<CrearInvitacionDatos>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    validar_rol_workspace(&datos.role)?;
    workspace_existe(&pool, datos.workspace_id).await?;
    let email = servicio::normalizar_email(&datos.email)?;

    let token = tokens::generar_token_opaco();
    let expira = Utc::now() + chrono::Duration::hours(tokens::DURACION_INVITACION_HORAS);

    sqlx::query!(
        "INSERT INTO workspace_invitations
             (workspace_id, invited_email, role, token, expires_at, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)",
        datos.workspace_id,
        email,
        datos.role,
        tokens::hash_token(&token),
        expira,
        dev.id
    )
    .execute(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(datos.workspace_id),
        Some(dev.id),
        acciones::INVITACION_CREADA,
        json!({"workspace_id": datos.workspace_id, "email": email, "role": datos.role}),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "mensaje": "Invitación creada; comparte el token por un canal seguro",
            "token": token,
            "expira": expira,
        })),
    ))
}

// ------------------------- GET /admin/auditoria -------------------------

#[derive(Deserialize)]
pub struct PaginacionAuditoria {
    /// Cuántas filas devolver (máx. 200).
    pub limite: Option<i64>,
    /// Cuántas filas saltar (para paginar).
    pub desplazamiento: Option<i64>,
}

/// A diferencia de `movimientos::Movimiento` (ya scoped a un
/// workspace), esta vista es global y cruza tenants — por eso también
/// resuelve `workspace_name`, útil para saber de qué tenant es cada
/// entrada.
#[derive(Serialize)]
pub struct EntradaAuditoria {
    pub id: Uuid,
    pub actor_name: String,
    pub workspace_name: Option<String>,
    pub action: String,
    pub detail: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<Utc>,
}

/// Lee la bitácora de auditoría, de lo más reciente a lo más viejo.
pub async fn listar_auditoria(
    State(pool): State<PgPool>,
    SoloDev(_dev): SoloDev,
    Query(paginacion): Query<PaginacionAuditoria>,
) -> Result<Json<Vec<EntradaAuditoria>>, AppError> {
    let limite = paginacion.limite.unwrap_or(50).clamp(1, 200);
    let desplazamiento = paginacion.desplazamiento.unwrap_or(0).max(0);

    let filas = sqlx::query_as!(
        EntradaAuditoria,
        r#"SELECT a.id, COALESCE(u.name, 'Sistema') AS "actor_name!", w.name AS "workspace_name?",
                  a.action, a.detail, a.created_at
           FROM audit_log a
           LEFT JOIN users u ON u.id = a.user_id
           LEFT JOIN workspaces w ON w.id = a.workspace_id
           ORDER BY a.created_at DESC
           LIMIT $1 OFFSET $2"#,
        limite,
        desplazamiento
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(filas))
}

// ------------------------- DELETE /admin/workspaces/:id/miembros/:user_id -------------------------

/// Quita a un usuario de un workspace. No se puede eliminar al
/// propietario (`workspaces.owner_id`) por esta vía — el owner no se
/// gestiona como una membresía más.
pub async fn eliminar_miembro(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Path((workspace_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM workspaces WHERE id = $1",
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Workspace no encontrado".to_string()))?;

    if owner_id == user_id {
        return Err(AppError::NoProcesable(
            "No se puede eliminar al propietario del workspace".to_string(),
        ));
    }

    let resultado = sqlx::query!(
        "DELETE FROM workspace_members WHERE workspace_id = $1 AND user_id = $2",
        workspace_id,
        user_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado("Miembro no encontrado".to_string()));
    }

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(dev.id),
        acciones::MIEMBRO_ELIMINADO,
        json!({"workspace_id": workspace_id, "user_id": user_id}),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ------------------------- DELETE /admin/workspaces/:id -------------------------

/// Borra un tenant y TODA su información (transacciones, cuentas,
/// transferencias, categorías propias, etiquetas, suscripciones,
/// previstos, metas, inversiones, membresías, notificaciones,
/// invitaciones) en un solo `DELETE`: todas las tablas cuelgan de
/// `workspaces` (directa o transitivamente) con `ON DELETE CASCADE`
/// (ver docs/database.md), así que Postgres se encarga de la cascada
/// completa. Operación irreversible — el frontend exige confirmación
/// explícita antes de llamar a este endpoint.
pub async fn eliminar_workspace(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Path(workspace_id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let nombre = sqlx::query_scalar!(
        "DELETE FROM workspaces WHERE id = $1 RETURNING name",
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Workspace no encontrado".to_string()))?;

    // `workspace_id: None` a propósito: `audit_log.workspace_id`
    // también tiene `ON DELETE CASCADE`, así que auditar con
    // `Some(workspace_id)` haría que esta misma fila se autodestruya
    // en la cascada que acaba de correr — quedaría sin rastro de que
    // el tenant se borró.
    auditoria::registrar(
        &pool,
        None,
        Some(dev.id),
        acciones::WORKSPACE_ELIMINADO,
        json!({"workspace_id": workspace_id, "name": nombre}),
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}

// ------------------------- POST /admin/usuarios/:id/desactivar -------------------------

/// Desactiva una cuenta (bloquea el login) y revoca sus refresh tokens
/// vigentes, para que pierda la sesión en el siguiente refresh.
pub async fn desactivar_usuario(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Path(id): Path<Uuid>,
) -> Result<Json<RespuestaUsuario>, AppError> {
    if id == dev.id {
        return Err(AppError::NoProcesable(
            "No puedes desactivar tu propia cuenta".to_string(),
        ));
    }

    let usuario = sqlx::query_as!(
        User,
        "UPDATE users SET is_active = false WHERE id = $1
         RETURNING id, name, email, password_hash, role, is_active,
                   failed_login_attempts, locked_until, created_at",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Usuario no encontrado".to_string()))?;

    tokens::revocar_todos_los_refresh(&pool, id).await?;

    auditoria::registrar(
        &pool,
        None,
        Some(dev.id),
        acciones::USUARIO_DESACTIVADO,
        json!({"user_id": id}),
    )
    .await;

    Ok(Json(usuario.into()))
}

// ------------------------- POST /admin/usuarios/:id/reactivar -------------------------

/// Reactiva una cuenta previamente desactivada.
pub async fn reactivar_usuario(
    State(pool): State<PgPool>,
    SoloDev(dev): SoloDev,
    Path(id): Path<Uuid>,
) -> Result<Json<RespuestaUsuario>, AppError> {
    let usuario = sqlx::query_as!(
        User,
        "UPDATE users SET is_active = true WHERE id = $1
         RETURNING id, name, email, password_hash, role, is_active,
                   failed_login_attempts, locked_until, created_at",
        id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Usuario no encontrado".to_string()))?;

    auditoria::registrar(
        &pool,
        None,
        Some(dev.id),
        acciones::USUARIO_REACTIVADO,
        json!({"user_id": id}),
    )
    .await;

    Ok(Json(usuario.into()))
}
