// =====================================================================
// suscripciones.rs — Gastos fijos recurrentes (internet, streaming...).
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Months, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::categorias::validar_categoria;
use crate::accounting::models::{
    ActualizarSuscripcionDatos, CrearSuscripcionDatos, FiltroProximosCobros, FiltrosSuscripciones,
    Suscripcion,
};
use crate::accounts::validar_cuenta_propia;
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

/// Cuánto (y en qué sentido) mueve el saldo de la cuenta al marcar una
/// suscripción como cobrada: siempre negativo, una suscripción es por
/// definición un gasto — mismo criterio que
/// `accounting::transacciones::ajuste_balance`.
fn ajuste_balance(monto: Decimal) -> Decimal {
    -monto
}

const PERIODICIDADES: [&str; 4] = ["monthly", "bimonthly", "quarterly", "annual"];

fn validar_periodicidad(periodicity: &str) -> Result<(), AppError> {
    if PERIODICIDADES.contains(&periodicity) {
        Ok(())
    } else {
        Err(AppError::NoProcesable(format!(
            "La periodicidad debe ser una de: {}",
            PERIODICIDADES.join(", ")
        )))
    }
}

fn validar_monto(monto: Decimal) -> Result<(), AppError> {
    if monto > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El monto debe ser mayor a cero".to_string(),
        ))
    }
}

/// Cuántos meses avanza cada periodicidad al marcar un cobro como hecho.
fn meses_por_periodicidad(periodicity: &str) -> u32 {
    match periodicity {
        "monthly" => 1,
        "bimonthly" => 2,
        "quarterly" => 3,
        "annual" => 12,
        // Imposible: el CHECK de la tabla subscriptions solo permite
        // estos cuatro valores, así que cualquier fila leída de la DB
        // ya viene garantizada.
        _ => unreachable!("periodicidad ya validada por el CHECK de la tabla"),
    }
}

/// Si la categoría viene, confirma que es visible y de tipo 'expense'
/// (una suscripción es, por definición, un gasto).
async fn validar_categoria_opcional(
    pool: &PgPool,
    workspace_id: Uuid,
    category_id: Option<Uuid>,
) -> Result<(), AppError> {
    if let Some(category_id) = category_id {
        validar_categoria(pool, category_id, workspace_id, "expense").await?;
    }
    Ok(())
}

/// POST /workspaces/:workspace_id/suscripciones
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearSuscripcionDatos>,
) -> Result<(StatusCode, Json<Suscripcion>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_periodicidad(&datos.periodicity)?;
    validar_monto(datos.amount)?;
    validar_categoria_opcional(&pool, workspace_id, datos.category_id).await?;
    if let Some(account_id) = datos.account_id {
        validar_cuenta_propia(&pool, account_id, workspace_id, usuario.id).await?;
    }

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let fila = sqlx::query_as!(
        Suscripcion,
        r#"INSERT INTO subscriptions
               (workspace_id, owner_id, name, amount, category_id, periodicity, next_billing_date, account_id)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, workspace_id, owner_id, name, amount, category_id, periodicity,
                     next_billing_date, is_active, created_at, account_id"#,
        workspace_id,
        usuario.id,
        datos.name.trim(),
        datos.amount,
        datos.category_id,
        datos.periodicity,
        datos.next_billing_date,
        datos.account_id
    )
    .fetch_one(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::SUSCRIPCION_CREADA,
        json!({"suscripcion_id": fila.id, "name": fila.name}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/suscripciones?activas=true|false
///
/// Un `member` solo ve las suyas; un `admin`/dev ve todas (supervisión).
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosSuscripciones>,
) -> Result<Json<Vec<Suscripcion>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let filas = sqlx::query_as!(
        Suscripcion,
        r#"SELECT id, workspace_id, owner_id, name, amount, category_id, periodicity,
                  next_billing_date, is_active, created_at, account_id
           FROM subscriptions
           WHERE workspace_id = $1
             AND ($2::bool IS NULL OR is_active = $2)
             AND ($3::uuid IS NULL OR owner_id = $3)
           ORDER BY is_active DESC, next_billing_date"#,
        workspace_id,
        filtros.activas,
        solo_propias
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// GET /workspaces/:workspace_id/suscripciones/proximos-cobros?dias=30
///
/// Suscripciones activas cuyo próximo cobro cae dentro de la ventana
/// indicada (30 días por defecto). Mismo criterio personal/supervisión
/// que `listar`.
pub async fn proximos_cobros(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroProximosCobros>,
) -> Result<Json<Vec<Suscripcion>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let dias = filtro.dias.unwrap_or(30).clamp(1, 365);
    let limite: NaiveDate = Utc::now().date_naive() + chrono::Duration::days(dias);

    let filas = sqlx::query_as!(
        Suscripcion,
        r#"SELECT id, workspace_id, owner_id, name, amount, category_id, periodicity,
                  next_billing_date, is_active, created_at, account_id
           FROM subscriptions
           WHERE workspace_id = $1 AND is_active = true AND next_billing_date <= $2
             AND ($3::uuid IS NULL OR owner_id = $3)
           ORDER BY next_billing_date"#,
        workspace_id,
        limite,
        solo_propias
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// PUT /workspaces/:workspace_id/suscripciones/:id — reemplazo
/// completo. Solo el dueño, sin excepción de rol.
pub async fn actualizar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<ActualizarSuscripcionDatos>,
) -> Result<Json<Suscripcion>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_periodicidad(&datos.periodicity)?;
    validar_monto(datos.amount)?;
    validar_categoria_opcional(&pool, workspace_id, datos.category_id).await?;
    if let Some(account_id) = datos.account_id {
        validar_cuenta_propia(&pool, account_id, workspace_id, usuario.id).await?;
    }

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM subscriptions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Suscripción no encontrada".to_string()))?;

    if owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño de la suscripción puede editarla".to_string(),
        ));
    }

    let fila = sqlx::query_as!(
        Suscripcion,
        r#"UPDATE subscriptions
           SET name = $1, amount = $2, category_id = $3, periodicity = $4,
               next_billing_date = $5, is_active = $6, account_id = $7
           WHERE id = $8 AND workspace_id = $9
           RETURNING id, workspace_id, owner_id, name, amount, category_id, periodicity,
                     next_billing_date, is_active, created_at, account_id"#,
        datos.name.trim(),
        datos.amount,
        datos.category_id,
        datos.periodicity,
        datos.next_billing_date,
        datos.is_active,
        datos.account_id,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Suscripción no encontrada".to_string()))?;

    // Las transacciones que ya generó esta suscripción (marcar_cobrada)
    // quedan vinculadas por subscription_id — se corrige su categoría
    // también, para que las métricas reflejen el cambio de inmediato.
    // No se propaga monto/cuenta: cambiar eso implicaría re-ajustar
    // saldos ya movidos.
    sqlx::query!(
        "UPDATE transactions SET category_id = $1 WHERE subscription_id = $2 AND is_active = true",
        fila.category_id,
        id
    )
    .execute(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::SUSCRIPCION_EDITADA,
        json!({"suscripcion_id": fila.id}),
    )
    .await;

    Ok(Json(fila))
}

/// POST /workspaces/:workspace_id/suscripciones/:id/marcar-cobrada
///
/// Avanza `next_billing_date` según la periodicidad, para reflejar que
/// el cobro de este ciclo ya ocurrió. Si la suscripción tiene cuenta
/// asignada, además genera el movimiento real: bloquea esa cuenta,
/// resta el monto de su saldo e inserta la transacción — mismo patrón
/// que `transacciones::crear`, todo en una sola transacción de BD. La
/// fecha de la transacción es el `next_billing_date` **viejo** (el
/// cobro que efectivamente correspondía a ese ciclo), no la fecha en
/// que se marca cobrada. Sin cuenta asignada, se mantiene el
/// comportamiento anterior: solo avanza la fecha.
pub async fn marcar_cobrada(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Suscripcion>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let mut tx = pool.begin().await?;

    let actual = sqlx::query_as!(
        Suscripcion,
        r#"SELECT id, workspace_id, owner_id, name, amount, category_id, periodicity,
                  next_billing_date, is_active, created_at, account_id
           FROM subscriptions WHERE id = $1 AND workspace_id = $2 FOR UPDATE"#,
        id,
        workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Suscripción no encontrada".to_string()))?;

    if actual.owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño de la suscripción puede marcarla cobrada".to_string(),
        ));
    }

    let meses = meses_por_periodicidad(&actual.periodicity);
    let siguiente = actual
        .next_billing_date
        .checked_add_months(Months::new(meses))
        .ok_or_else(|| AppError::Interno("Overflow al calcular la próxima fecha".to_string()))?;

    if let Some(account_id) = actual.account_id {
        let cuenta = sqlx::query_scalar!(
            "SELECT id FROM accounts WHERE id = $1 AND workspace_id = $2 FOR UPDATE",
            account_id,
            workspace_id
        )
        .fetch_optional(&mut *tx)
        .await?;
        if cuenta.is_none() {
            return Err(AppError::NoEncontrado("Cuenta no encontrada".to_string()));
        }

        sqlx::query!(
            "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
            ajuste_balance(actual.amount),
            account_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            r#"INSERT INTO transactions
                   (workspace_id, type, amount, date, category_id, account_id, description,
                    created_by, subscription_id)
               VALUES ($1, 'expense', $2, $3, $4, $5, $6, $7, $8)"#,
            workspace_id,
            actual.amount,
            actual.next_billing_date,
            actual.category_id,
            account_id,
            actual.name,
            usuario.id,
            id
        )
        .execute(&mut *tx)
        .await?;
    }

    let fila = sqlx::query_as!(
        Suscripcion,
        r#"UPDATE subscriptions SET next_billing_date = $1
           WHERE id = $2
           RETURNING id, workspace_id, owner_id, name, amount, category_id, periodicity,
                     next_billing_date, is_active, created_at, account_id"#,
        siguiente,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::SUSCRIPCION_COBRADA,
        json!({"suscripcion_id": fila.id, "next_billing_date": fila.next_billing_date}),
    )
    .await;

    Ok(Json(fila))
}

/// DELETE /workspaces/:workspace_id/suscripciones/:id — solo el dueño.
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM subscriptions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Suscripción no encontrada".to_string()))?;

    if owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño de la suscripción puede eliminarla".to_string(),
        ));
    }

    let resultado = sqlx::query!(
        "DELETE FROM subscriptions WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado(
            "Suscripción no encontrada".to_string(),
        ));
    }
    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::SUSCRIPCION_ELIMINADA,
        json!({"suscripcion_id": id}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
