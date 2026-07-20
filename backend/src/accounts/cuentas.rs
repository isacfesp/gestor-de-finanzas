// =====================================================================
// cuentas.rs — Cuentas y billeteras del workspace (efectivo, banco...).
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounts::models::{
    ActualizarCuentaDatos, AlertaTarjeta, CrearCuentaDatos, Cuenta, FiltroAlertasTarjeta,
    FiltrosCuentas, MiembroBasico,
};
use crate::auditoria::{self, acciones};
use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

const TIPOS_CUENTA: [&str; 4] = ["cash", "debit", "credit", "savings"];

/// Días de anticipación para avisar de una fecha de corte/pago próxima.
const DIAS_AVISO_TARJETA: i64 = 7;

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

/// Mismo criterio que `normalizar_credit_limit`: solo las tarjetas de
/// crédito tienen día de corte/pago, y ambos son obligatorios (1-31)
/// para ese tipo. Para el resto de los tipos se ignoran, así una cuenta
/// que deja de ser `credit` no se queda con fechas huérfanas.
fn normalizar_dias_facturacion(
    tipo: &str,
    cutoff_day: Option<i16>,
    payment_due_day: Option<i16>,
) -> Result<(Option<i16>, Option<i16>), AppError> {
    if tipo != "credit" {
        return Ok((None, None));
    }
    match (cutoff_day, payment_due_day) {
        (Some(corte), Some(pago)) if (1..=31).contains(&corte) && (1..=31).contains(&pago) => {
            Ok((Some(corte), Some(pago)))
        }
        _ => Err(AppError::NoProcesable(
            "Las tarjetas de crédito requieren día de corte y día límite de pago entre 1 y 31"
                .to_string(),
        )),
    }
}

/// Último día real de `mes` en `anio` (28-31), calculado como el día
/// anterior al primero del mes siguiente.
fn ultimo_dia_del_mes(anio: i32, mes: u32) -> u32 {
    let (anio_siguiente, mes_siguiente) = if mes == 12 {
        (anio + 1, 1)
    } else {
        (anio, mes + 1)
    };
    NaiveDate::from_ymd_opt(anio_siguiente, mes_siguiente, 1)
        .expect("mes+1 normalizado siempre es una fecha válida")
        .pred_opt()
        .expect("el día anterior al 1 de cualquier mes siempre existe")
        .day()
}

/// Próxima ocurrencia (inclusive) de `dia` como día del mes, contada
/// desde `hoy`. Si `dia` no existe en el mes evaluado (ej. 31 en
/// febrero), usa el último día real de ese mes.
pub(crate) fn proxima_ocurrencia_dia_mes(hoy: NaiveDate, dia: u32) -> NaiveDate {
    let candidato_este_mes = dia.min(ultimo_dia_del_mes(hoy.year(), hoy.month()));
    let fecha_este_mes = hoy
        .with_day(candidato_este_mes)
        .expect("candidato ya está acotado al último día real del mes");

    if fecha_este_mes >= hoy {
        return fecha_este_mes;
    }

    let (anio_siguiente, mes_siguiente) = if hoy.month() == 12 {
        (hoy.year() + 1, 1)
    } else {
        (hoy.year(), hoy.month() + 1)
    };
    let candidato_siguiente = dia.min(ultimo_dia_del_mes(anio_siguiente, mes_siguiente));
    NaiveDate::from_ymd_opt(anio_siguiente, mes_siguiente, candidato_siguiente)
        .expect("candidato ya está acotado al último día real del mes siguiente")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dia_31_en_febrero_usa_el_ultimo_dia_real() {
        let hoy = NaiveDate::from_ymd_opt(2026, 2, 1).unwrap();
        assert_eq!(
            proxima_ocurrencia_dia_mes(hoy, 31),
            NaiveDate::from_ymd_opt(2026, 2, 28).unwrap()
        );
    }

    #[test]
    fn dia_31_en_febrero_bisiesto() {
        let hoy = NaiveDate::from_ymd_opt(2028, 2, 1).unwrap();
        assert_eq!(
            proxima_ocurrencia_dia_mes(hoy, 31),
            NaiveDate::from_ymd_opt(2028, 2, 29).unwrap()
        );
    }

    #[test]
    fn hoy_cae_justo_en_el_dia_devuelve_hoy() {
        let hoy = NaiveDate::from_ymd_opt(2026, 7, 15).unwrap();
        assert_eq!(proxima_ocurrencia_dia_mes(hoy, 15), hoy);
    }

    #[test]
    fn rollover_normal_de_mes() {
        let hoy = NaiveDate::from_ymd_opt(2026, 7, 20).unwrap();
        assert_eq!(
            proxima_ocurrencia_dia_mes(hoy, 5),
            NaiveDate::from_ymd_opt(2026, 8, 5).unwrap()
        );
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
    let (cutoff_day, payment_due_day) =
        normalizar_dias_facturacion(&datos.tipo, datos.cutoff_day, datos.payment_due_day)?;
    let balance_inicial = datos.balance.unwrap_or(Decimal::ZERO);
    let moneda = datos.currency.unwrap_or_else(|| "MXN".to_string());

    let resultado = sqlx::query_as!(
        Cuenta,
        r#"INSERT INTO accounts
               (workspace_id, owner_id, name, type, balance, currency, credit_limit,
                cutoff_day, payment_due_day)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
           RETURNING id, workspace_id, owner_id, name, type AS "tipo", balance, currency,
                     is_active, credit_limit, cutoff_day, payment_due_day, created_at"#,
        workspace_id,
        usuario.id,
        datos.name.trim(),
        datos.tipo,
        balance_inicial,
        moneda,
        credit_limit,
        cutoff_day,
        payment_due_day
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
                  is_active, credit_limit, cutoff_day, payment_due_day, created_at
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
    let (cutoff_day, payment_due_day) =
        normalizar_dias_facturacion(&datos.tipo, datos.cutoff_day, datos.payment_due_day)?;

    let resultado = sqlx::query_as!(
        Cuenta,
        r#"UPDATE accounts
           SET name = $1, type = $2, currency = $3, is_active = $4, credit_limit = $5,
               cutoff_day = $6, payment_due_day = $7
           WHERE id = $8 AND workspace_id = $9
           RETURNING id, workspace_id, owner_id, name, type AS "tipo", balance, currency,
                     is_active, credit_limit, cutoff_day, payment_due_day, created_at"#,
        datos.name.trim(),
        datos.tipo,
        datos.currency,
        datos.is_active,
        credit_limit,
        cutoff_day,
        payment_due_day,
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

/// GET /workspaces/:workspace_id/cuentas/alertas-tarjeta?dias=N
///
/// Tarjetas de crédito propias (o de todo el workspace si es admin/dev)
/// cuya próxima fecha de corte o de pago límite cae dentro de los
/// próximos `dias` (7 por defecto) — a diferencia de la campana de
/// notificaciones, siempre refleja el estado actual, sin depender de si
/// ya se generó o leyó un aviso.
pub async fn alertas_tarjeta(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroAlertasTarjeta>,
) -> Result<Json<Vec<AlertaTarjeta>>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    let dias = filtro.dias.unwrap_or(DIAS_AVISO_TARJETA).clamp(1, 90);
    let hoy = Utc::now().date_naive();
    let limite = hoy + chrono::Duration::days(dias);

    let cuentas = sqlx::query!(
        r#"SELECT id, name, currency, balance, credit_limit AS "credit_limit!",
                  cutoff_day AS "cutoff_day!", payment_due_day AS "payment_due_day!"
           FROM accounts
           WHERE workspace_id = $1 AND type = 'credit' AND is_active = true
             AND cutoff_day IS NOT NULL AND payment_due_day IS NOT NULL
             AND ($2::uuid IS NULL OR owner_id = $2)"#,
        workspace_id,
        solo_propias
    )
    .fetch_all(&pool)
    .await?;

    let alertas = cuentas
        .into_iter()
        .filter_map(|cuenta| {
            let corte = proxima_ocurrencia_dia_mes(hoy, cuenta.cutoff_day as u32);
            let pago = proxima_ocurrencia_dia_mes(hoy, cuenta.payment_due_day as u32);
            if corte > limite && pago > limite {
                return None;
            }
            Some(AlertaTarjeta {
                account_id: cuenta.id,
                account_name: cuenta.name,
                currency: cuenta.currency,
                balance: cuenta.balance,
                credit_limit: cuenta.credit_limit,
                cutoff_date: corte,
                payment_due_date: pago,
            })
        })
        .collect();

    Ok(Json(alertas))
}
