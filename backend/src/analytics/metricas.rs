// =====================================================================
// metricas.rs — Cash flow, tasa de ahorro y distribución de gastos.
// Todo son GET de solo lectura: no hay entidad que crear/editar/borrar.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::analytics::comun::resolver_filtro_usuario;
use crate::analytics::models::{
    AhorroNeto, DistribucionGasto, FiltroMes, FiltroPeriodo, FlujoCaja, TasaAhorro,
};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

/// La convención del proyecto es que `month` siempre sea el día 1
/// (ver `accounting::presupuestos::primer_dia_del_mes`).
fn primer_dia_del_mes(fecha: NaiveDate) -> NaiveDate {
    fecha
        .with_day(1)
        .expect("el día 1 siempre es válido en cualquier mes")
}

/// GET /workspaces/:workspace_id/analytics/flujo-caja?desde=&hasta=
pub async fn flujo_caja(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroPeriodo>,
) -> Result<Json<FlujoCaja>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let fila = sqlx::query!(
        r#"SELECT
               COALESCE(SUM(amount) FILTER (WHERE type = 'income'), 0) AS "income!",
               COALESCE(SUM(amount) FILTER (WHERE type = 'expense'), 0) AS "expense!"
           FROM transactions
           WHERE workspace_id = $1 AND is_active = true
             AND ($2::date IS NULL OR date >= $2)
             AND ($3::date IS NULL OR date <= $3)
             AND ($4::uuid IS NULL OR created_by = $4)"#,
        workspace_id,
        filtro.desde,
        filtro.hasta,
        filtro_usuario
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(FlujoCaja {
        desde: filtro.desde,
        hasta: filtro.hasta,
        income: fila.income,
        expense: fila.expense,
        net: fila.income - fila.expense,
    }))
}

/// GET /workspaces/:workspace_id/analytics/ahorro-neto?desde=&hasta=
///
/// Dinero nuevo que entró a metas en el rango: aportes (ingreso) menos
/// retiros (egreso), ambos con `goal_id` — monto absoluto, a
/// diferencia de `tasa_ahorro` que es un porcentaje del mes en curso.
pub async fn ahorro_neto(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroPeriodo>,
) -> Result<Json<AhorroNeto>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let fila = sqlx::query!(
        r#"SELECT
               COALESCE(SUM(amount) FILTER (WHERE type = 'income'), 0) AS "aportado!",
               COALESCE(SUM(amount) FILTER (WHERE type = 'expense'), 0) AS "retirado!"
           FROM transactions
           WHERE workspace_id = $1 AND is_active = true AND goal_id IS NOT NULL
             AND ($2::date IS NULL OR date >= $2)
             AND ($3::date IS NULL OR date <= $3)
             AND ($4::uuid IS NULL OR created_by = $4)"#,
        workspace_id,
        filtro.desde,
        filtro.hasta,
        filtro_usuario
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(AhorroNeto {
        desde: filtro.desde,
        hasta: filtro.hasta,
        aportado: fila.aportado,
        retirado: fila.retirado,
        neto: fila.aportado - fila.retirado,
    }))
}

/// GET /workspaces/:workspace_id/analytics/tasa-ahorro?month=YYYY-MM-DD
pub async fn tasa_ahorro(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroMes>,
) -> Result<Json<TasaAhorro>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let mes = filtro
        .month
        .map(primer_dia_del_mes)
        .unwrap_or_else(|| primer_dia_del_mes(Utc::now().date_naive()));

    let fila = sqlx::query!(
        r#"SELECT
               COALESCE(SUM(amount) FILTER (WHERE goal_id IS NOT NULL), 0) AS "goal_income!",
               COALESCE(SUM(amount), 0) AS "total_income!"
           FROM transactions
           WHERE workspace_id = $1 AND type = 'income' AND is_active = true
             AND date_trunc('month', date)::date = $2
             AND ($3::uuid IS NULL OR created_by = $3)"#,
        workspace_id,
        mes,
        filtro_usuario
    )
    .fetch_one(&pool)
    .await?;

    let percentage = if fila.total_income.is_zero() {
        Decimal::ZERO
    } else {
        fila.goal_income * Decimal::from(100) / fila.total_income
    };

    Ok(Json(TasaAhorro {
        month: mes,
        total_income: fila.total_income,
        goal_income: fila.goal_income,
        percentage,
    }))
}

/// GET /workspaces/:workspace_id/analytics/distribucion-gastos?desde=&hasta=
pub async fn distribucion_gastos(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroPeriodo>,
) -> Result<Json<Vec<DistribucionGasto>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let filas = sqlx::query_as!(
        DistribucionGasto,
        r#"WITH gastos AS (
               SELECT t.category_id, COALESCE(c.name, 'Sin categoría') AS category_name,
                      SUM(t.amount) AS monto
               FROM transactions t
               LEFT JOIN categories c ON c.id = t.category_id
               WHERE t.workspace_id = $1 AND t.type = 'expense' AND t.is_active = true
                 AND ($2::date IS NULL OR t.date >= $2)
                 AND ($3::date IS NULL OR t.date <= $3)
                 AND ($4::uuid IS NULL OR t.created_by = $4)
               GROUP BY t.category_id, c.name
           )
           SELECT category_id, category_name AS "category_name!", monto AS "amount!",
                  (monto * 100 / SUM(monto) OVER ()) AS "percentage!"
           FROM gastos
           ORDER BY monto DESC"#,
        workspace_id,
        filtro.desde,
        filtro.hasta,
        filtro_usuario
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}
