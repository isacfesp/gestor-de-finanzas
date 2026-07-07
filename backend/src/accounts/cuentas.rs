// =====================================================================
// cuentas.rs — Cuentas y billeteras del workspace (efectivo, banco...).
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::models::{ActualizarCuentaDatos, CrearCuentaDatos, Cuenta, FiltrosCuentas};
use crate::auth::autorizacion::verificar_membresia;
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

/// Confirma que `account_id` existe y pertenece a `workspace_id`. La usan
/// otros módulos (transferencias, previstos) antes de referenciar una
/// cuenta ajena.
pub(crate) async fn validar_cuenta(
    pool: &PgPool,
    account_id: Uuid,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM accounts WHERE id = $1 AND workspace_id = $2)",
        account_id,
        workspace_id
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

    let balance_inicial = datos.balance.unwrap_or(rust_decimal::Decimal::ZERO);
    let moneda = datos.currency.unwrap_or_else(|| "MXN".to_string());

    let resultado = sqlx::query_as!(
        Cuenta,
        r#"INSERT INTO accounts (workspace_id, name, type, balance, currency)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING id, workspace_id, name, type AS "tipo", balance, currency,
                     is_active, created_at"#,
        workspace_id,
        datos.name.trim(),
        datos.tipo,
        balance_inicial,
        moneda
    )
    .fetch_one(&pool)
    .await;

    match resultado {
        Ok(cuenta) => Ok((StatusCode::CREATED, Json(cuenta))),
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
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosCuentas>,
) -> Result<Json<Vec<Cuenta>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Cuenta,
        r#"SELECT id, workspace_id, name, type AS "tipo", balance, currency,
                  is_active, created_at
           FROM accounts
           WHERE workspace_id = $1
             AND ($2::bool IS NULL OR is_active = $2)
           ORDER BY name"#,
        workspace_id,
        filtros.activas
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// PUT /workspaces/:workspace_id/cuentas/:id — no toca el balance.
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

    let resultado = sqlx::query_as!(
        Cuenta,
        r#"UPDATE accounts
           SET name = $1, type = $2, currency = $3, is_active = $4
           WHERE id = $5 AND workspace_id = $6
           RETURNING id, workspace_id, name, type AS "tipo", balance, currency,
                     is_active, created_at"#,
        datos.name.trim(),
        datos.tipo,
        datos.currency,
        datos.is_active,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await;

    match resultado {
        Ok(Some(cuenta)) => Ok(Json(cuenta)),
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

/// DELETE /workspaces/:workspace_id/cuentas/:id
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

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
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        // La cuenta está referenciada por transferencias o previstos.
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23503") => Err(
            AppError::Conflicto("La cuenta está en uso, no se puede eliminar".to_string()),
        ),
        Err(e) => Err(e.into()),
    }
}
