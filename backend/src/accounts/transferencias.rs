// =====================================================================
// transferencias.rs — Movimiento de dinero entre cuentas del workspace.
//
// No es ingreso ni egreso: mueve el balance de una cuenta a otra dentro
// de una sola transacción de base de datos, para que nunca quede el
// dinero "descontado" de una cuenta sin haberse acreditado en la otra.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::models::{CrearTransferenciaDatos, FiltrosTransferencias, Transferencia};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

fn validar_monto(monto: Decimal) -> Result<(), AppError> {
    if monto > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El monto debe ser mayor a cero".to_string(),
        ))
    }
}

/// POST /workspaces/:workspace_id/transferencias
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearTransferenciaDatos>,
) -> Result<(StatusCode, Json<Transferencia>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_monto(datos.amount)?;

    if datos.from_account_id == datos.to_account_id {
        return Err(AppError::NoProcesable(
            "La cuenta de origen y destino no pueden ser la misma".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;

    // Se bloquean las dos cuentas siempre en el mismo orden (por id) para
    // que dos transferencias concurrentes entre las mismas cuentas nunca
    // se bloqueen mutuamente (deadlock).
    let (primera, segunda) = if datos.from_account_id < datos.to_account_id {
        (datos.from_account_id, datos.to_account_id)
    } else {
        (datos.to_account_id, datos.from_account_id)
    };

    let cuentas_bloqueadas = sqlx::query!(
        r#"SELECT id FROM accounts
           WHERE id IN ($1, $2) AND workspace_id = $3
           ORDER BY id
           FOR UPDATE"#,
        primera,
        segunda,
        workspace_id
    )
    .fetch_all(&mut *tx)
    .await?;

    if cuentas_bloqueadas.len() != 2 {
        return Err(AppError::NoEncontrado(
            "Alguna de las cuentas no existe en este workspace".to_string(),
        ));
    }

    sqlx::query!(
        "UPDATE accounts SET balance = balance - $1 WHERE id = $2",
        datos.amount,
        datos.from_account_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
        datos.amount,
        datos.to_account_id
    )
    .execute(&mut *tx)
    .await?;

    let transferencia = sqlx::query_as!(
        Transferencia,
        r#"INSERT INTO transfers
               (workspace_id, from_account_id, to_account_id, amount, date, description, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING id, workspace_id, from_account_id, to_account_id, amount, date,
                     description, created_by, created_at"#,
        workspace_id,
        datos.from_account_id,
        datos.to_account_id,
        datos.amount,
        datos.date,
        datos.description,
        usuario.id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(transferencia)))
}

/// GET /workspaces/:workspace_id/transferencias — con filtros opcionales.
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosTransferencias>,
) -> Result<Json<Vec<Transferencia>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Transferencia,
        r#"SELECT id, workspace_id, from_account_id, to_account_id, amount, date,
                  description, created_by, created_at
           FROM transfers
           WHERE workspace_id = $1
             AND ($2::date IS NULL OR date >= $2)
             AND ($3::date IS NULL OR date <= $3)
             AND ($4::uuid IS NULL OR from_account_id = $4 OR to_account_id = $4)
           ORDER BY date DESC, created_at DESC"#,
        workspace_id,
        filtros.desde,
        filtros.hasta,
        filtros.account_id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}
