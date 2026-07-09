// =====================================================================
// handlers.rs — Endpoints de autenticación.
//
// Principio de todo este archivo: las respuestas de error NUNCA
// revelan información. "Credenciales inválidas" se responde igual si
// el email no existe, si la contraseña está mal o si la cuenta está
// desactivada — un atacante no puede aprender nada probando.
// =====================================================================

use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auditoria::{self, acciones};
use crate::auth::extractores::UsuarioAutenticado;
use crate::auth::jwt::{DURACION_ACCESS_MIN, emitir_access_token};
use crate::auth::tokens;
use crate::errores::AppError;
use crate::users::models::RespuestaUsuario;
use crate::users::servicio;

/// Intentos fallidos consecutivos antes de bloquear la cuenta.
const MAX_INTENTOS_FALLIDOS: i32 = 5;
/// Minutos que dura el bloqueo por fuerza bruta.
const MINUTOS_DE_BLOQUEO: i64 = 15;

// ------------------------- Tipos de request/response -------------------------

#[derive(Deserialize)]
pub struct LoginDatos {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct RefreshDatos {
    pub refresh_token: String,
}

#[derive(Deserialize)]
pub struct AceptarInvitacionDatos {
    pub token: String,
    pub name: String,
    pub password: String,
}

/// Par de tokens que recibe el cliente al iniciar sesión o refrescar.
#[derive(Serialize)]
pub struct TokensRespuesta {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    /// Segundos de vida del access token, para que el cliente sepa
    /// cuándo refrescar sin decodificar el JWT.
    pub expires_in: i64,
}

/// Arma la respuesta estándar de tokens para un usuario.
async fn emitir_par_de_tokens(
    pool: &PgPool,
    user_id: uuid::Uuid,
    rol: &str,
) -> Result<TokensRespuesta, AppError> {
    Ok(TokensRespuesta {
        access_token: emitir_access_token(user_id, rol)?,
        refresh_token: tokens::crear_refresh_token(pool, user_id).await?,
        token_type: "Bearer",
        expires_in: DURACION_ACCESS_MIN * 60,
    })
}

/// El error genérico de login. Función para garantizar que el mensaje
/// sea idéntico en todos los caminos de fallo.
fn credenciales_invalidas() -> AppError {
    AppError::NoAutorizado("Credenciales inválidas".to_string())
}

// ------------------------- POST /auth/login -------------------------

/// Verifica email + contraseña y entrega access + refresh token.
///
/// Contra fuerza bruta: 5 fallos consecutivos bloquean la cuenta 15
/// minutos (429). Todo intento queda en la bitácora de auditoría.
pub async fn login(
    State(pool): State<PgPool>,
    Json(datos): Json<LoginDatos>,
) -> Result<Json<TokensRespuesta>, AppError> {
    // Limpieza oportunista de tokens muy viejos; si falla no afecta al login.
    let _ = tokens::purgar_refresh_vencidos(&pool).await;

    let mut conexion = pool.acquire().await?;
    // Un email con formato inválido (Err) se trata igual que uno
    // inexistente (None): mismo mensaje genérico.
    let usuario = servicio::buscar_por_email(&mut conexion, &datos.email)
        .await
        .unwrap_or_default();

    let Some(usuario) = usuario else {
        // El email no existe. Se calcula un bcrypt de todos modos para
        // que esta ruta tarde lo mismo que una contraseña incorrecta:
        // si respondiera más rápido, el tiempo de respuesta delataría
        // qué emails están registrados (timing attack).
        let _ = bcrypt::hash(&datos.password, bcrypt::DEFAULT_COST);
        auditoria::registrar(
            &pool,
            None,
            None,
            acciones::LOGIN_FALLIDO,
            json!({"email": datos.email}),
        )
        .await;
        return Err(credenciales_invalidas());
    };

    // ¿Cuenta bloqueada por intentos fallidos? (if-let encadenado con &&:
    // entra solo si locked_until tiene valor Y ese valor es futuro)
    if let Some(bloqueada_hasta) = usuario.locked_until
        && bloqueada_hasta > Utc::now()
    {
        auditoria::registrar(
            &pool,
            None,
            Some(usuario.id),
            acciones::LOGIN_BLOQUEADO,
            json!({}),
        )
        .await;
        return Err(AppError::DemasiadosIntentos);
    }

    if !usuario.is_active {
        auditoria::registrar(
            &pool,
            None,
            Some(usuario.id),
            acciones::LOGIN_FALLIDO,
            json!({"motivo": "cuenta_inactiva"}),
        )
        .await;
        return Err(credenciales_invalidas());
    }

    // Comparación segura: bcrypt::verify hashea la contraseña recibida
    // y compara contra el hash guardado. La contraseña en texto plano
    // jamás se guarda ni se loggea.
    if !bcrypt::verify(&datos.password, &usuario.password_hash)? {
        // Contraseña incorrecta: sumar el intento y bloquear si llegó al límite.
        sqlx::query!(
            "UPDATE users SET
                failed_login_attempts = failed_login_attempts + 1,
                locked_until = CASE
                    WHEN failed_login_attempts + 1 >= $2 THEN now() + make_interval(mins => $3)
                    ELSE locked_until
                END
             WHERE id = $1",
            usuario.id,
            MAX_INTENTOS_FALLIDOS,
            MINUTOS_DE_BLOQUEO as i32
        )
        .execute(&pool)
        .await?;

        auditoria::registrar(
            &pool,
            None,
            Some(usuario.id),
            acciones::LOGIN_FALLIDO,
            json!({"intentos": usuario.failed_login_attempts + 1}),
        )
        .await;
        return Err(credenciales_invalidas());
    }

    // Login correcto: reiniciar el contador de fallos y quitar bloqueos.
    sqlx::query!(
        "UPDATE users SET failed_login_attempts = 0, locked_until = NULL WHERE id = $1",
        usuario.id
    )
    .execute(&pool)
    .await?;

    let respuesta = emitir_par_de_tokens(&pool, usuario.id, &usuario.role).await?;
    auditoria::registrar(&pool, None, Some(usuario.id), acciones::LOGIN_OK, json!({})).await;
    Ok(Json(respuesta))
}

// ------------------------- POST /auth/refresh -------------------------

/// Cambia un refresh token válido por un par nuevo (rotación).
///
/// El token usado queda revocado: cada refresh token sirve UNA vez.
/// Si llega un token que ya fue usado o venció, se asume posible robo
/// y se cierran TODAS las sesiones del usuario.
pub async fn refresh(
    State(pool): State<PgPool>,
    Json(datos): Json<RefreshDatos>,
) -> Result<Json<TokensRespuesta>, AppError> {
    let Some(guardado) = tokens::buscar_refresh_token(&pool, &datos.refresh_token).await? else {
        return Err(AppError::NoAutorizado("Sesión inválida".to_string()));
    };

    if guardado.expires_at <= Utc::now() {
        // Token conocido pero ya usado/vencido → posible token robado.
        // Medida drástica y segura: cerrar todas las sesiones del usuario.
        tokens::revocar_todos_los_refresh(&pool, guardado.user_id).await?;
        auditoria::registrar(
            &pool,
            None,
            Some(guardado.user_id),
            acciones::REFRESH_REUSO,
            json!({}),
        )
        .await;
        return Err(AppError::NoAutorizado("Sesión inválida".to_string()));
    }

    // El rol se relee de la base en cada refresh: si el dev desactiva
    // una cuenta o le cambia el rol, surte efecto en minutos (cuando
    // expire el access token), sin esperar 30 días de refresh.
    let cuenta = sqlx::query!(
        "SELECT role, is_active FROM users WHERE id = $1",
        guardado.user_id
    )
    .fetch_one(&pool)
    .await?;

    if !cuenta.is_active {
        tokens::revocar_todos_los_refresh(&pool, guardado.user_id).await?;
        return Err(AppError::NoAutorizado("Sesión inválida".to_string()));
    }

    // Rotación: revocar el token usado y emitir un par nuevo.
    tokens::revocar_refresh_token(&pool, guardado.id).await?;
    let respuesta = emitir_par_de_tokens(&pool, guardado.user_id, &cuenta.role).await?;
    Ok(Json(respuesta))
}

// ------------------------- POST /auth/logout -------------------------

/// Cierra la sesión revocando el refresh token recibido.
/// (El access token no se puede revocar, pero muere solo en ≤15 min.)
pub async fn logout(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Json(datos): Json<RefreshDatos>,
) -> Result<StatusCode, AppError> {
    if let Some(guardado) = tokens::buscar_refresh_token(&pool, &datos.refresh_token).await? {
        // Solo puedes revocar tus propios tokens.
        if guardado.user_id == usuario.id {
            tokens::revocar_refresh_token(&pool, guardado.id).await?;
        }
    }
    auditoria::registrar(&pool, None, Some(usuario.id), acciones::LOGOUT, json!({})).await;
    Ok(StatusCode::NO_CONTENT)
}

// ------------------------- GET /auth/yo -------------------------

/// Devuelve los datos del usuario autenticado (sin campos sensibles).
pub async fn yo(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
) -> Result<Json<RespuestaUsuario>, AppError> {
    let fila = sqlx::query_as!(
        RespuestaUsuario,
        "SELECT id, name, email, role, is_active, created_at FROM users WHERE id = $1",
        usuario.id
    )
    .fetch_one(&pool)
    .await?;
    Ok(Json(fila))
}

// ------------------------- GET /auth/mis-workspaces -------------------------

/// Workspace visible para el usuario autenticado, sin los campos que
/// solo le hacen falta al panel de administración (`created_at`,
/// conteo de miembros).
#[derive(Serialize)]
pub struct WorkspaceResumen {
    pub id: Uuid,
    pub name: String,
}

/// Los workspaces del usuario autenticado: un dev ve todos los tenants
/// (bypasa la tabla de membresía, igual que en el resto del sistema);
/// un usuario normal solo los que tienen una fila en
/// `workspace_members`. Resuelve el pendiente anotado en `CLAUDE.md`
/// ("no existe un endpoint para que un usuario normal liste sus
/// propios workspaces").
pub async fn mis_workspaces(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
) -> Result<Json<Vec<WorkspaceResumen>>, AppError> {
    let filas = if usuario.es_dev() {
        sqlx::query_as!(
            WorkspaceResumen,
            "SELECT id, name FROM workspaces ORDER BY created_at"
        )
        .fetch_all(&pool)
        .await?
    } else {
        sqlx::query_as!(
            WorkspaceResumen,
            r#"SELECT w.id, w.name FROM workspaces w
               JOIN workspace_members m ON m.workspace_id = w.id
               WHERE m.user_id = $1
               ORDER BY w.name"#,
            usuario.id
        )
        .fetch_all(&pool)
        .await?
    };
    Ok(Json(filas))
}

// ------------------------- POST /auth/invitaciones/aceptar -------------------------

/// Canjea un link de invitación: crea la cuenta (si el email no existía)
/// y la une al workspace con el rol indicado en la invitación.
///
/// Toda la operación va en UNA transacción: si algo falla a medias, no
/// queda ni el usuario creado ni la invitación quemada.
pub async fn aceptar_invitacion(
    State(pool): State<PgPool>,
    Json(datos): Json<AceptarInvitacionDatos>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let hash = tokens::hash_token(&datos.token);

    let invitacion = sqlx::query!(
        "SELECT id, workspace_id, invited_email, role, expires_at, used_at
         FROM workspace_invitations WHERE token = $1",
        hash
    )
    .fetch_optional(&pool)
    .await?;

    // Inválida, ya usada o vencida → mismo mensaje en los tres casos.
    let invitacion = match invitacion {
        Some(i) if i.used_at.is_none() && i.expires_at > Utc::now() => i,
        _ => {
            return Err(AppError::NoAutorizado(
                "Invitación inválida o expirada".to_string(),
            ));
        }
    };

    let mut tx = pool.begin().await?;

    // Si el email invitado ya tiene cuenta se reutiliza (ignorando el
    // nombre/contraseña del request); si no, se crea como 'usuario'.
    let usuario = match servicio::buscar_por_email(&mut tx, &invitacion.invited_email).await? {
        Some(existente) => existente,
        None => {
            servicio::crear_usuario(
                &mut tx,
                &datos.name,
                &invitacion.invited_email,
                &datos.password,
                "usuario",
            )
            .await?
        }
    };

    // ON CONFLICT: si ya era miembro, la invitación no duplica ni pisa su rol.
    sqlx::query!(
        "INSERT INTO workspace_members (workspace_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (workspace_id, user_id) DO NOTHING",
        invitacion.workspace_id,
        usuario.id,
        invitacion.role
    )
    .execute(&mut *tx)
    .await?;

    // Quemar la invitación: un solo uso.
    sqlx::query!(
        "UPDATE workspace_invitations SET used_at = now() WHERE id = $1",
        invitacion.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    auditoria::registrar(
        &pool,
        Some(invitacion.workspace_id),
        Some(usuario.id),
        acciones::INVITACION_ACEPTADA,
        json!({"workspace_id": invitacion.workspace_id}),
    )
    .await;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "mensaje": "Invitación aceptada, ya puedes iniciar sesión",
            "workspace_id": invitacion.workspace_id,
        })),
    ))
}
