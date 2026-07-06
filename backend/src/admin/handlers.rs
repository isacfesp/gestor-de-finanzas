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
use crate::users::models::RespuestaUsuario;
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
        Some(dev.id),
        acciones::USUARIO_CREADO,
        json!({"nuevo_usuario_id": usuario.id, "email": usuario.email}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(usuario.into())))
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

#[derive(Serialize)]
pub struct EntradaAuditoria {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
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
        "SELECT id, user_id, action, detail, created_at
         FROM audit_log ORDER BY created_at DESC
         LIMIT $1 OFFSET $2",
        limite,
        desplazamiento
    )
    .fetch_all(&pool)
    .await?;
    Ok(Json(filas))
}
