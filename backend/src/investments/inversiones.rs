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
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
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
               (workspace_id, owner_id, name, principal, gat_annual_rate, interest_type,
                start_date, term_days, end_date)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING id, workspace_id, owner_id, name, principal, gat_annual_rate, interest_type,
                     start_date, term_days, end_date, is_active, created_at"#,
        workspace_id,
        usuario.id,
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

    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::INVERSION_CREADA,
        json!({"inversion_id": fila.id, "name": fila.name}),
    )
    .await;

    Ok((StatusCode::CREATED, Json(fila)))
}

/// GET /workspaces/:workspace_id/inversiones?activas=true|false
///
/// Un `member` solo ve las suyas; un `admin`/dev ve todas (supervisión).
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtros): Query<FiltrosInversiones>,
) -> Result<Json<Vec<Inversion>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let filas = sqlx::query_as!(
        Inversion,
        r#"SELECT id, workspace_id, owner_id, name, principal, gat_annual_rate, interest_type,
                  start_date, term_days, end_date, is_active, created_at
           FROM investments
           WHERE workspace_id = $1
             AND ($2::bool IS NULL OR is_active = $2)
             AND ($3::uuid IS NULL OR owner_id = $3)
           ORDER BY start_date DESC"#,
        workspace_id,
        filtros.activas,
        solo_propias
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
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let fila = sqlx::query_as!(
        Inversion,
        r#"SELECT id, workspace_id, owner_id, name, principal, gat_annual_rate, interest_type,
                  start_date, term_days, end_date, is_active, created_at
           FROM investments
           WHERE id = $1 AND workspace_id = $2
             AND ($3::uuid IS NULL OR owner_id = $3)"#,
        id,
        workspace_id,
        solo_propias
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Inversión no encontrada".to_string()))?;

    Ok(Json(fila))
}

/// DELETE /workspaces/:workspace_id/inversiones/:id — borra en cascada
/// su historial de rendimientos (investment_yields). Solo el dueño,
/// sin excepción de rol (admin/dev solo supervisan, no borran lo ajeno).
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let owner_id = sqlx::query_scalar!(
        "SELECT owner_id FROM investments WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .fetch_optional(&pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Inversión no encontrada".to_string()))?;

    if owner_id != usuario.id {
        return Err(AppError::Prohibido(
            "Solo el dueño de la inversión puede eliminarla".to_string(),
        ));
    }

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
    auditoria::registrar(
        &pool,
        Some(workspace_id),
        Some(usuario.id),
        acciones::INVERSION_ELIMINADA,
        json!({"inversion_id": id}),
    )
    .await;
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
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let inversion = sqlx::query!(
        r#"SELECT principal, gat_annual_rate, interest_type, term_days
           FROM investments
           WHERE id = $1 AND workspace_id = $2
             AND ($3::uuid IS NULL OR owner_id = $3)"#,
        id,
        workspace_id,
        solo_propias
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
