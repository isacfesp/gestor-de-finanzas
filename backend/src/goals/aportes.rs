// =====================================================================
// aportes.rs — Vincular fondos a una meta y consultar su progreso.
//
// Un aporte es, en la práctica, una transacción (ingreso o retiro)
// marcada con goal_id: se inserta en `transactions` y se refleja de
// inmediato en `goals.current_amount`, todo dentro de una sola
// transacción de base de datos para que ambas tablas nunca queden
// desincronizadas.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::goals::models::{
    Aporte, AporteDatos, FiltroProyeccion, Meta, ProgresoMeta, ProyeccionMeta,
};

fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    if tipo == "income" || tipo == "expense" {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El tipo debe ser 'income' o 'expense'".to_string(),
        ))
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

/// POST /workspaces/:workspace_id/metas/:id/aportes
///
/// Un aporte mueve dinero real de/hacia una cuenta: 'income' resta de
/// la cuenta (el dinero se aparta hacia el ahorro), 'expense' (retiro)
/// le devuelve el monto — sentido opuesto al que aplica sobre
/// `goals.current_amount` más abajo. Misma cuenta bloqueada con `FOR
/// UPDATE` dentro de la transacción, igual que
/// `accounting::transacciones::crear`.
pub async fn vincular(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Json(datos): Json<AporteDatos>,
) -> Result<Json<Meta>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_monto(datos.amount)?;
    let tipo = datos.tipo.unwrap_or_else(|| "income".to_string());
    validar_tipo(&tipo)?;

    let mut tx = pool.begin().await?;

    let cuenta = sqlx::query_scalar!(
        "SELECT id FROM accounts WHERE id = $1 AND workspace_id = $2 FOR UPDATE",
        datos.account_id,
        workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?;
    if cuenta.is_none() {
        return Err(AppError::NoEncontrado("Cuenta no encontrada".to_string()));
    }

    // FOR UPDATE: si dos aportes a la misma meta llegan a la vez, el
    // segundo espera a que el primero termine de leer y escribir
    // current_amount, evitando que uno pise el resultado del otro.
    let actual = sqlx::query!(
        r#"SELECT target_amount, current_amount FROM goals
           WHERE id = $1 AND workspace_id = $2
           FOR UPDATE"#,
        id,
        workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Meta no encontrada".to_string()))?;

    let delta = if tipo == "income" {
        datos.amount
    } else {
        -datos.amount
    };
    let nuevo_monto = actual.current_amount + delta;

    if nuevo_monto < Decimal::ZERO {
        return Err(AppError::NoProcesable(
            "El retiro supera los fondos acumulados en la meta".to_string(),
        ));
    }

    let completada = nuevo_monto >= actual.target_amount;

    let meta = sqlx::query_as!(
        Meta,
        r#"UPDATE goals SET current_amount = $1, is_completed = $2
           WHERE id = $3
           RETURNING id, workspace_id, name, target_amount, current_amount,
                     deadline, is_completed, created_at"#,
        nuevo_monto,
        completada,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    let ajuste_cuenta = if tipo == "income" {
        -datos.amount
    } else {
        datos.amount
    };
    sqlx::query!(
        "UPDATE accounts SET balance = balance + $1 WHERE id = $2",
        ajuste_cuenta,
        datos.account_id
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO transactions
               (workspace_id, type, amount, date, description, goal_id, account_id, created_by)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        workspace_id,
        tipo,
        datos.amount,
        datos.date,
        datos.description,
        id,
        datos.account_id,
        usuario.id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::APORTE_REGISTRADO,
        json!({"meta_id": id, "amount": datos.amount, "type": tipo}),
    )
    .await;

    Ok(Json(meta))
}

/// GET /workspaces/:workspace_id/metas/:id/aportes — historial de
/// transacciones vinculadas a esta meta, más reciente primero.
pub async fn listar_aportes(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Aporte>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    sqlx::query!(
        "SELECT id FROM goals WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Meta no encontrada".to_string()))?;

    let filas = sqlx::query_as!(
        Aporte,
        r#"SELECT t.id, t.type AS "tipo", t.amount, t.date, t.description,
                  u.name AS created_by_name
           FROM transactions t
           JOIN users u ON u.id = t.created_by
           WHERE t.goal_id = $1 AND t.is_active = true
           ORDER BY t.date DESC, t.created_at DESC"#,
        id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// GET /workspaces/:workspace_id/metas/:id/progreso
pub async fn progreso(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<ProgresoMeta>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let fila = sqlx::query_as!(
        ProgresoMeta,
        r#"SELECT id, name, target_amount, current_amount,
                  GREATEST(target_amount - current_amount, 0) AS "remaining_amount!",
                  (current_amount * 100 / target_amount) AS "percentage!",
                  deadline, is_completed
           FROM goals
           WHERE id = $1 AND workspace_id = $2"#,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Meta no encontrada".to_string()))?;

    Ok(Json(fila))
}

/// Cuántos períodos completos (semanas o meses) quedan entre `hoy` y
/// `deadline`. Se aproxima el mes a 30 días: alcanza para calcular un
/// aporte periódico razonable sin manejar meses de distinta longitud.
fn periodos_restantes(hoy: NaiveDate, deadline: NaiveDate, periodo: &str) -> Result<i64, AppError> {
    let dias_restantes = (deadline - hoy).num_days();
    if dias_restantes <= 0 {
        return Err(AppError::NoProcesable(
            "La fecha límite de la meta ya pasó".to_string(),
        ));
    }

    let dias_por_periodo = if periodo == "weekly" { 7 } else { 30 };
    // División hacia arriba: un período parcial cuenta como completo.
    Ok((dias_restantes + dias_por_periodo - 1) / dias_por_periodo)
}

/// GET /workspaces/:workspace_id/metas/:id/proyeccion?periodo=weekly|monthly
pub async fn proyeccion(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
    Query(filtro): Query<FiltroProyeccion>,
) -> Result<Json<ProyeccionMeta>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let periodo = filtro.periodo.unwrap_or_else(|| "monthly".to_string());
    if periodo != "weekly" && periodo != "monthly" {
        return Err(AppError::NoProcesable(
            "El período debe ser 'weekly' o 'monthly'".to_string(),
        ));
    }

    let meta = sqlx::query!(
        "SELECT target_amount, current_amount, deadline FROM goals WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Meta no encontrada".to_string()))?;

    let restante = (meta.target_amount - meta.current_amount).max(Decimal::ZERO);
    let hoy = Utc::now().date_naive();
    let periodos = periodos_restantes(hoy, meta.deadline, &periodo)?;

    let aporte_necesario = restante / Decimal::from(periodos);

    Ok(Json(ProyeccionMeta {
        periodo,
        periodos_restantes: periodos,
        aporte_necesario,
    }))
}
