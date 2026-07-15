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
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::models::{
    CrearTransferenciaDatos, FiltrosTransferencias, Transferencia, TransferenciaListado,
};
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
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

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::TRANSFERENCIA_CREADA,
        json!({
            "transferencia_id": transferencia.id,
            "from_account_id": transferencia.from_account_id,
            "to_account_id": transferencia.to_account_id,
            "amount": transferencia.amount,
        }),
    )
    .await;

    Ok((StatusCode::CREATED, Json(transferencia)))
}

/// GET /workspaces/:workspace_id/transferencias — con filtros opcionales.
///
/// Un `member` solo ve las transferencias donde alguna de sus cuentas
/// participa (origen o destino); un `admin`/dev ve todas (supervisión).
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosTransferencias>,
) -> Result<Json<Vec<TransferenciaListado>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let filas = sqlx::query_as!(
        TransferenciaListado,
        r#"SELECT t.id, t.workspace_id,
                  t.from_account_id, origen.name AS from_account_name,
                  t.to_account_id, destino.name AS to_account_name,
                  t.amount, t.date, t.description, t.created_by, t.created_at
           FROM transfers t
           JOIN accounts origen ON origen.id = t.from_account_id
           JOIN accounts destino ON destino.id = t.to_account_id
           WHERE t.workspace_id = $1
             AND ($2::date IS NULL OR t.date >= $2)
             AND ($3::date IS NULL OR t.date <= $3)
             AND ($4::uuid IS NULL OR t.from_account_id = $4 OR t.to_account_id = $4)
             AND ($5::uuid IS NULL OR origen.owner_id = $5 OR destino.owner_id = $5)
           ORDER BY t.date DESC, t.created_at DESC"#,
        workspace_id,
        filtros.desde,
        filtros.hasta,
        filtros.account_id,
        solo_propias
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}
