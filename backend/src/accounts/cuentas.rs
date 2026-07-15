// =====================================================================
// cuentas.rs — Cuentas y billeteras del workspace (efectivo, banco...).
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::models::{
    ActualizarCuentaDatos, CrearCuentaDatos, Cuenta, FiltrosCuentas, MiembroBasico,
};
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

const TIPOS_CUENTA: [&str; 5] = ["cash", "debit", "credit", "savings", "investment"];

fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    if TIPOS_CUENTA.contains(&tipo) {
        Ok(())
    } else {
        Err(AppError::NoProcesable(format!(
            "El tipo debe ser una de: {}",
            TIPOS_CUENTA.join(", ")
        )))
    }
}

/// Las tarjetas de crédito son el único tipo con límite: lo exige mayor
/// a cero. Para el resto de los tipos lo ignora (siempre `None`) sin
/// importar lo que haya mandado el cliente — así una cuenta que deja de
/// ser `credit` no se queda con un límite huérfano.
fn normalizar_credit_limit(
    tipo: &str,
    credit_limit: Option<Decimal>,
) -> Result<Option<Decimal>, AppError> {
    if tipo != "credit" {
        return Ok(None);
    }
    match credit_limit {
        Some(limite) if limite > Decimal::ZERO => Ok(Some(limite)),
        _ => Err(AppError::NoProcesable(
            "Las tarjetas de crédito requieren un límite mayor a cero".to_string(),
        )),
    }
}

/// Confirma que `account_id` existe en `workspace_id` Y pertenece a
/// `owner_id`. Operar sobre la cuenta de otro se trata igual que
/// "cuenta inexistente" (las cuentas son personales).
pub(crate) async fn validar_cuenta_propia(
    pool: &PgPool,
    account_id: Uuid,
    workspace_id: Uuid,
    owner_id: Uuid,
) -> Result<(), AppError> {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1 AND workspace_id = $2 AND owner_id = $3)",
        account_id,
        workspace_id,
        owner_id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(false);

    if existe {
        Ok(())
    } else {
        Err(AppError::NoEncontrado("Cuenta no encontrada".to_string()))
    }
}

/// POST /workspaces/:workspace_id/cuentas
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearCuentaDatos>,
) -> Result<(StatusCode, Json<Cuenta>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_tipo(&datos.tipo)?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let credit_limit = normalizar_credit_limit(&datos.tipo, datos.credit_limit)?;
    let balance_inicial = datos.balance.unwrap_or(Decimal::ZERO);
    let moneda = datos.currency.unwrap_or_else(|| "MXN".to_string());

    let resultado = sqlx::query_as!(
        Cuenta,
        r#"INSERT INTO accounts (workspace_id, owner_id, name, type, balance, currency, credit_limit)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, workspace_id, owner_id, name, type AS "tipo", balance, currency,
                     is_active, credit_limit, created_at"#,
        workspace_id,
        usuario.id,
        datos.name.trim(),
        datos.tipo,
        balance_inicial,
        moneda,
        credit_limit
    )
    .fetch_one(&pool)
    .await;

    match resultado {
        Ok(cuenta) => {
            auditoria::registrar(
                &pool,
                Some(workspace_id),
                Some(usuario.id),
                acciones::CUENTA_CREADA,
                json!({"cuenta_id": cuenta.id, "name": cuenta.name}),
            )
            .await;
            Ok((StatusCode::CREATED, Json(cuenta)))
        }
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("accounts_workspace_name_unique") =>
        {
            Err(AppError::Conflicto(
                "Ya existe una cuenta con ese nombre en este workspace".to_string(),
            ))
        }
        Err(e) => Err(e.into()),
    }
}

/// GET /workspaces/:workspace_id/cuentas?activas=true|false
///
/// Un `member` solo ve sus propias cuentas (ni se entera de que
/// existen otras); un `admin`/dev ve todas, para supervisión.
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosCuentas>,
) -> Result<Json<Vec<Cuenta>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let filas = sqlx::query_as!(
        Cuenta,
        r#"SELECT id, workspace_id, owner_id, name, type AS "tipo", balance, currency,
                  is_active, credit_limit, created_at
           FROM accounts
           WHERE workspace_id = $1
             AND ($2::bool IS NULL OR is_active = $2)
             AND ($3::uuid IS NULL OR owner_id = $3)
           ORDER BY name"#,
        workspace_id,
        filtros.activas,
        solo_propias
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// GET /workspaces/:workspace_id/cuentas/miembros
///
/// Nombre e id de cada miembro del workspace, solo para que un
/// admin/dev arme el mapa "de quién es esta cuenta" en la vista de
/// supervisión. Un member recibe 403: no necesita conocer a sus
/// compañeros de workspace para operar sus propias cuentas.
pub async fn listar_miembros(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<MiembroBasico>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    if matches!(rol, RolWorkspace::Member) {
        return Err(AppError::Prohibido(
            "Solo un admin puede ver la lista de miembros".to_string(),
        ));
    }

    let filas = sqlx::query_as!(
        MiembroBasico,
        r#"SELECT u.id AS user_id, u.name
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

/// PUT /workspaces/:workspace_id/cuentas/:id — no toca el balance.
///
/// Solo el dueño puede editar su cuenta: ni el admin del workspace ni
/// un dev pasan por encima de esto (su rol solo les da supervisión de
/// lectura, ver `listar`).
pub async fn actualizar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<ActualizarCuentaDatos>,
) -> Result<Json<Cuenta>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_tipo(&datos.tipo)?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM accounts WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Cuenta no encontrada".to_string()))?;

    if owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño de la cuenta puede editarla".to_string(),
        ));
    }

    let credit_limit = normalizar_credit_limit(&datos.tipo, datos.credit_limit)?;

    let resultado = sqlx::query_as!(
        Cuenta,
        r#"UPDATE accounts
           SET name = $1, type = $2, currency = $3, is_active = $4, credit_limit = $5
           WHERE id = $6 AND workspace_id = $7
           RETURNING id, workspace_id, owner_id, name, type AS "tipo", balance, currency,
                     is_active, credit_limit, created_at"#,
        datos.name.trim(),
        datos.tipo,
        datos.currency,
        datos.is_active,
        credit_limit,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await;

    match resultado {
        Ok(Some(cuenta)) => {
            auditoria::registrar(
                &pool,
                Some(workspace_id),
                Some(usuario.id),
                acciones::CUENTA_EDITADA,
                json!({"cuenta_id": cuenta.id, "name": cuenta.name}),
            )
            .await;
            Ok(Json(cuenta))
        }
        Ok(None) => Err(AppError::NoEncontrado("Cuenta no encontrada".to_string())),
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("accounts_workspace_name_unique") =>
        {
            Err(AppError::Conflicto(
                "Ya existe una cuenta con ese nombre en este workspace".to_string(),
            ))
        }
        Err(e) => Err(e.into()),
    }
}

/// DELETE /workspaces/:workspace_id/cuentas/:id — solo el dueño.
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM accounts WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Cuenta no encontrada".to_string()))?;

    if owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño de la cuenta puede eliminarla".to_string(),
        ));
    }

    let resultado = sqlx::query!(
        "DELETE FROM accounts WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await;

    match resultado {
        Ok(r) if r.rows_affected() == 0 => {
            Err(AppError::NoEncontrado("Cuenta no encontrada".to_string()))
        }
        Ok(_) => {
            auditoria::registrar(
                &pool,
                Some(workspace_id),
                Some(usuario.id),
                acciones::CUENTA_ELIMINADA,
                json!({"cuenta_id": id}),
            )
            .await;
            Ok(StatusCode::NO_CONTENT)
        }
        // La cuenta está referenciada por transferencias o previstos.
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23503") => Err(
            AppError::Conflicto("La cuenta está en uso, no se puede eliminar".to_string()),
        ),
        Err(e) => Err(e.into()),
    }
}
