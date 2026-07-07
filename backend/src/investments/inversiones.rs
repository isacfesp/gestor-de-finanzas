// =====================================================================
// inversiones.rs — CRUD de inversiones, proyección de rendimiento e
// ISR, y simulador sin persistencia.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::Days;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::investments::calculos::{
    calcular_desglose, validar_plazo, validar_principal, validar_tasa, validar_tipo_interes,
};
use crate::investments::models::{
    CrearInversionDatos, DesgloseRendimiento, FiltrosInversiones, Inversion, SimularInversionDatos,
};

/// POST /workspaces/:workspace_id/inversiones
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearInversionDatos>,
) -> Result<(StatusCode, Json<Inversion>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_tipo_interes(&datos.interest_type)?;
    validar_principal(datos.principal)?;
    validar_tasa(datos.gat_annual_rate)?;
    validar_plazo(datos.term_days)?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let end_date = datos
        .start_date
        .checked_add_days(Days::new(datos.term_days as u64))
        .ok_or_else(|| AppError::NoProcesable("El plazo produce una fecha inválida".to_string()))?;

    let fila = sqlx::query_as!(
        Inversion,
        r#"INSERT INTO investments
               (workspace_id, name, principal, gat_annual_rate, interest_type,
                start_date, term_days, end_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, workspace_id, name, principal, gat_annual_rate, interest_type,
                     start_date, term_days, end_date, is_active, created_at"#,
        workspace_id,
        datos.name.trim(),
        datos.principal,
        datos.gat_annual_rate,
        datos.interest_type,
        datos.start_date,
        datos.term_days,
        end_date
    )
    .fetch_one(&pool)
    .await?;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/inversiones?activas=true|false
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosInversiones>,
) -> Result<Json<Vec<Inversion>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Inversion,
        r#"SELECT id, workspace_id, name, principal, gat_annual_rate, interest_type,
                  start_date, term_days, end_date, is_active, created_at
           FROM investments
           WHERE workspace_id = $1
             AND ($2::bool IS NULL OR is_active = $2)
           ORDER BY start_date DESC"#,
        workspace_id,
        filtros.activas
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// GET /workspaces/:workspace_id/inversiones/:id
pub async fn obtener(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Inversion>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let fila = sqlx::query_as!(
        Inversion,
        r#"SELECT id, workspace_id, name, principal, gat_annual_rate, interest_type,
                  start_date, term_days, end_date, is_active, created_at
           FROM investments WHERE id = $1 AND workspace_id = $2"#,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Inversión no encontrada".to_string()))?;

    Ok(Json(fila))
}

/// DELETE /workspaces/:workspace_id/inversiones/:id — borra en cascada
/// su historial de rendimientos (investment_yields).
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let resultado = sqlx::query!(
        "DELETE FROM investments WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await?;

    if resultado.rows_affected() == 0 {
        return Err(AppError::NoEncontrado(
            "Inversión no encontrada".to_string(),
        ));
    }
    Ok(StatusCode::NO_CONTENT)
}

/// GET /workspaces/:workspace_id/inversiones/:id/proyeccion
///
/// Rendimiento bruto, ISR retenido, neto y monto al vencimiento de una
/// inversión ya registrada, con sus propios datos financieros.
pub async fn proyeccion(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DesgloseRendimiento>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let inversion = sqlx::query!(
        r#"SELECT principal, gat_annual_rate, interest_type, term_days
           FROM investments WHERE id = $1 AND workspace_id = $2"#,
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Inversión no encontrada".to_string()))?;

    let desglose = calcular_desglose(
        inversion.principal,
        inversion.gat_annual_rate,
        &inversion.interest_type,
        inversion.term_days,
    )?;

    Ok(Json(desglose))
}

/// POST /workspaces/:workspace_id/inversiones/simular — no persiste nada.
pub async fn simular(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<SimularInversionDatos>,
) -> Result<Json<DesgloseRendimiento>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_tipo_interes(&datos.interest_type)?;
    validar_principal(datos.principal)?;
    validar_tasa(datos.gat_annual_rate)?;
    validar_plazo(datos.term_days)?;

    let desglose = calcular_desglose(
        datos.principal,
        datos.gat_annual_rate,
        &datos.interest_type,
        datos.term_days,
    )?;

    Ok(Json(desglose))
}
