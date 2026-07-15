// =====================================================================
// presupuestos.rs — Límites de gasto mensuales por categoría.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::categorias::validar_categoria;
use crate::accounting::models::{CrearPresupuestoDatos, EstadoPresupuesto, FiltroMes, Presupuesto};
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

/// La convención del proyecto es que `month` siempre sea el día 1.
/// with_day(1) nunca falla: el día 1 existe en todos los meses.
fn primer_dia_del_mes(fecha: NaiveDate) -> NaiveDate {
    use chrono::Datelike;
    fecha
        .with_day(1)
        .expect("el día 1 siempre es válido en cualquier mes")
}

fn validar_monto(monto: Decimal) -> Result<(), AppError> {
    if monto > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El límite debe ser mayor a cero".to_string(),
        ))
    }
}

/// POST /workspaces/:workspace_id/presupuestos — crea o actualiza el
/// límite del mes para esa categoría (upsert).
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearPresupuestoDatos>,
) -> Result<(StatusCode, Json<Presupuesto>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_monto(datos.limit_amount)?;
    validar_categoria(&pool, datos.category_id, workspace_id, "expense").await?;

    let mes = primer_dia_del_mes(datos.month);

    let fila = sqlx::query_as!(
        Presupuesto,
        r#"INSERT INTO budgets (workspace_id, owner_id, category_id, month, limit_amount)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (workspace_id, owner_id, category_id, month)
               DO UPDATE SET limit_amount = EXCLUDED.limit_amount
           RETURNING id, workspace_id, owner_id, category_id, month, limit_amount"#,
        workspace_id,
        usuario.id,
        datos.category_id,
        mes,
        datos.limit_amount
    )
    .fetch_one(&pool)
    .await?;

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::PRESUPUESTO_GUARDADO,
        json!({"presupuesto_id": fila.id, "month": fila.month, "limit_amount": fila.limit_amount}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/presupuestos?month=YYYY-MM-DD
///
/// Un `member` solo ve los suyos; un `admin`/dev ve todos (supervisión).
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroMes>,
) -> Result<Json<Vec<Presupuesto>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propios = matches!(rol, RolWorkspace::Member).then_some(usuario.id);
    let mes = filtro.month.map(primer_dia_del_mes);

    let filas = sqlx::query_as!(
        Presupuesto,
        r#"SELECT id, workspace_id, owner_id, category_id, month, limit_amount
           FROM budgets
           WHERE workspace_id = $1
             AND ($2::date IS NULL OR month = $2)
             AND ($3::uuid IS NULL OR owner_id = $3)
           ORDER BY month DESC, category_id"#,
        workspace_id,
        mes,
        solo_propios
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// GET /workspaces/:workspace_id/presupuestos/estado?month=YYYY-MM-DD
///
/// Para cada presupuesto del mes, cuánto se ha gastado realmente en su
/// categoría (suma de transacciones tipo 'expense' de ese mes).
pub async fn estado(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroMes>,
) -> Result<Json<Vec<EstadoPresupuesto>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propios = matches!(rol, RolWorkspace::Member).then_some(usuario.id);
    let mes = filtro.month.map(primer_dia_del_mes).unwrap_or_else(|| {
        // Sin mes explícito: se asume el mes en curso.
        primer_dia_del_mes(Utc::now().date_naive())
    });

    // El gasto de cada presupuesto se compara solo contra las
    // transacciones de SU propio dueño (`t.created_by = b.owner_id`):
    // las transacciones también son personales, así que el gasto de un
    // miembro nunca debe contarse en el presupuesto de otro.
    let filas = sqlx::query_as!(
        EstadoPresupuesto,
        r#"SELECT b.id, b.owner_id, b.category_id, c.name AS category_name, b.month, b.limit_amount,
                  COALESCE(SUM(t.amount), 0) AS "spent!",
                  (COALESCE(SUM(t.amount), 0) * 100 / b.limit_amount) AS "percentage!"
           FROM budgets b
           JOIN categories c ON c.id = b.category_id
           LEFT JOIN transactions t
               ON t.category_id = b.category_id
              AND t.workspace_id = b.workspace_id
              AND t.created_by = b.owner_id
              AND t.type = 'expense'
              AND t.is_active = true
              AND date_trunc('month', t.date) = b.month
           WHERE b.workspace_id = $1 AND b.month = $2
             AND ($3::uuid IS NULL OR b.owner_id = $3)
           GROUP BY b.id, c.name
           ORDER BY c.name"#,
        workspace_id,
        mes,
        solo_propios
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// DELETE /workspaces/:workspace_id/presupuestos/:id — solo el dueño.
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM budgets WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Presupuesto no encontrado".to_string()))?;

    if owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño del presupuesto puede eliminarlo".to_string(),
        ));
    }

    let resultado = sqlx::query!(
        "DELETE FROM budgets WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado(
            "Presupuesto no encontrado".to_string(),
        ));
    }
    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::PRESUPUESTO_ELIMINADO,
        json!({"presupuesto_id": id}),
    )
    .await;
    Ok(StatusCode::NO_CONTENT)
}
