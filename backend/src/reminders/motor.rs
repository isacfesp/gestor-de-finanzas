// =====================================================================
// motor.rs — Ciclo en background: evalúa suscripciones y presupuestos
// de TODOS los workspaces y genera notificaciones. No es un handler
// HTTP: se lanza una sola vez desde main() con tokio::spawn y corre en
// loop mientras el proceso viva (no hay separación de proceso
// worker/servidor en este proyecto, ver docker-compose.yml).
// =====================================================================

use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde_json::json;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::accounting::suscripciones;
use crate::accounts::proxima_ocurrencia_dia_mes;
use crate::auditoria::{self, acciones};
use crate::correo;

/// Días de anticipación para avisar de un cobro próximo.
const DIAS_AVISO_SUSCRIPCION: i64 = 3;

/// Días de anticipación para avisar de una fecha de corte/pago próxima
/// de una tarjeta de crédito.
const DIAS_AVISO_TARJETA: i64 = 3;

/// Cada cuánto se re-evalúa todo. La spec pide un "ciclo diario"; se
/// aproxima con un intervalo fijo desde el arranque en vez de alinear
/// a medianoche exacta — no hay necesidad de negocio para esa
/// precisión en este proyecto.
const INTERVALO_CICLO: Duration = Duration::from_secs(60 * 60 * 24);

fn primer_dia_del_mes(fecha: NaiveDate) -> NaiveDate {
    fecha
        .with_day(1)
        .expect("el día 1 siempre es válido en cualquier mes")
}

/// Punto de entrada: corre para siempre, evaluando cada
/// `INTERVALO_CICLO`. Un error en una vuelta se loggea y no
/// interrumpe las siguientes — igual criterio que
/// `auditoria::registrar`: nunca tumbar el proceso por un fallo de
/// una tarea de fondo.
pub async fn ejecutar_ciclo_periodico(pool: PgPool) {
    loop {
        if let Err(e) = ejecutar_ciclo(&pool).await {
            eprintln!("ERROR en ciclo de recordatorios: {e}");
        }
        tokio::time::sleep(INTERVALO_CICLO).await;
    }
}

async fn ejecutar_ciclo(pool: &PgPool) -> Result<(), sqlx::Error> {
    // Primero: si una suscripción se cobra sola en esta vuelta, su
    // next_billing_date ya avanza antes de que revisar_suscripciones
    // evalúe si hay que avisar "por vencer" — evita notificar sobre un
    // cobro que ya se resolvió solo en el mismo ciclo.
    revisar_cobros_automaticos(pool).await?;
    revisar_suscripciones(pool).await?;
    revisar_presupuestos(pool).await?;
    revisar_tarjetas_credito(pool).await?;
    Ok(())
}

/// Suscripciones activas con cuenta asignada cuyo cobro ya venció:
/// ejecuta el mismo efecto que el botón manual "Marcar cobrada" (ver
/// `accounting::suscripciones::ejecutar_cobro`), sin intervención del
/// usuario. El botón manual sigue disponible como respaldo.
async fn revisar_cobros_automaticos(pool: &PgPool) -> Result<(), sqlx::Error> {
    let hoy = Utc::now().date_naive();

    let filas = sqlx::query!(
        r#"SELECT id, workspace_id FROM subscriptions
           WHERE is_active = true AND account_id IS NOT NULL AND next_billing_date <= $1"#,
        hoy
    )
    .fetch_all(pool)
    .await?;

    for fila in filas {
        match suscripciones::ejecutar_cobro(pool, fila.workspace_id, fila.id, None).await {
            Ok(cobrada) => {
                auditoria::registrar(
                    pool,
                    Some(fila.workspace_id),
                    None,
                    acciones::SUSCRIPCION_COBRADA_AUTOMATICA,
                    json!({"suscripcion_id": cobrada.id, "next_billing_date": cobrada.next_billing_date}),
                )
                .await;
            }
            // AppError solo deriva Debug, no Display: {:?}, nunca {}.
            Err(e) => eprintln!("ERROR en autocobro de la suscripción {}: {e:?}", fila.id),
        }
    }
    Ok(())
}

/// Tarjetas de crédito activas cuya próxima fecha de corte o de pago
/// límite cae dentro de `DIAS_AVISO_TARJETA`.
async fn revisar_tarjetas_credito(pool: &PgPool) -> Result<(), sqlx::Error> {
    let hoy = Utc::now().date_naive();

    let cuentas = sqlx::query!(
        r#"SELECT id, workspace_id, name, cutoff_day AS "cutoff_day!", payment_due_day AS "payment_due_day!"
           FROM accounts
           WHERE type = 'credit' AND is_active = true
             AND cutoff_day IS NOT NULL AND payment_due_day IS NOT NULL"#
    )
    .fetch_all(pool)
    .await?;

    for cuenta in cuentas {
        let corte = proxima_ocurrencia_dia_mes(hoy, cuenta.cutoff_day as u32);
        if corte <= hoy + chrono::Duration::days(DIAS_AVISO_TARJETA)
            && !ya_notificado(pool, "credit_card_cutoff", cuenta.id, true).await?
        {
            crear_notificacion(
                pool,
                cuenta.workspace_id,
                "credit_card_cutoff",
                &format!("Fecha de corte próxima: {}", cuenta.name),
                &format!("Tu tarjeta {} corta el {}.", cuenta.name, corte),
                cuenta.id,
            )
            .await?;
        }

        let pago = proxima_ocurrencia_dia_mes(hoy, cuenta.payment_due_day as u32);
        if pago <= hoy + chrono::Duration::days(DIAS_AVISO_TARJETA)
            && !ya_notificado(pool, "credit_card_due", cuenta.id, true).await?
        {
            crear_notificacion(
                pool,
                cuenta.workspace_id,
                "credit_card_due",
                &format!("Pago límite próximo: {}", cuenta.name),
                &format!("El pago límite de {} es el {}.", cuenta.name, pago),
                cuenta.id,
            )
            .await?;
        }
    }
    Ok(())
}

/// True si ya existe una notificación de este tipo+referencia. Con
/// `solo_no_leidas=true` solo cuenta las no leídas (para no repetir un
/// aviso mientras el usuario no lo haya atendido — caso de
/// suscripciones, cuyo `next_billing_date` no cambia hasta que se
/// marca cobrada); con `false` cuenta cualquiera (para presupuestos:
/// cada fila de `budgets` ya es única por mes, así que "ya existe"
/// alcanza para no reabrir el aviso ese mes).
async fn ya_notificado(
    pool: &PgPool,
    tipo: &str,
    reference_id: Uuid,
    solo_no_leidas: bool,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS(
               SELECT 1 FROM notifications
               WHERE type = $1 AND reference_id = $2 AND (NOT $3::bool OR is_read = false)
           )"#,
        tipo,
        reference_id,
        solo_no_leidas
    )
    .fetch_one(pool)
    .await
    .map(|v| v.unwrap_or(false))
}

async fn crear_notificacion(
    pool: &PgPool,
    workspace_id: Uuid,
    tipo: &str,
    title: &str,
    body: &str,
    reference_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO notifications (workspace_id, type, title, body, reference_id)
         VALUES ($1, $2, $3, $4, $5)",
        workspace_id,
        tipo,
        title,
        body,
        reference_id
    )
    .execute(pool)
    .await?;

    enviar_alerta_por_correo(pool, workspace_id, title, body).await;
    Ok(())
}

/// Envía la misma alerta por correo a cada miembro activo del
/// workspace. Best effort: un fallo de Resend (o de la consulta de
/// miembros) solo se loggea, nunca interrumpe el ciclo de recordatorios.
async fn enviar_alerta_por_correo(pool: &PgPool, workspace_id: Uuid, title: &str, body: &str) {
    let miembros = sqlx::query_scalar!(
        r#"SELECT u.email FROM workspace_members m
           JOIN users u ON u.id = m.user_id
           WHERE m.workspace_id = $1 AND u.is_active = true"#,
        workspace_id
    )
    .fetch_all(pool)
    .await;

    let miembros = match miembros {
        Ok(m) => m,
        Err(e) => {
            eprintln!("ERROR al buscar miembros del workspace {workspace_id} para alertar: {e}");
            return;
        }
    };

    for email in miembros {
        if let Err(e) = correo::enviar(&email, title, &correo::plantilla_alerta(title, body)).await
        {
            eprintln!("AVISO: no se pudo enviar la alerta por correo a {email}: {e}");
        }
    }
}

/// Suscripciones activas cuyo próximo cobro cae dentro de
/// `DIAS_AVISO_SUSCRIPCION` — misma condición que el endpoint de UI
/// `accounting::suscripciones::proximos_cobros`, pero sin filtrar por
/// workspace (recorre todos) y con una ventana fija más corta.
async fn revisar_suscripciones(pool: &PgPool) -> Result<(), sqlx::Error> {
    let limite = Utc::now().date_naive() + chrono::Duration::days(DIAS_AVISO_SUSCRIPCION);

    let filas = sqlx::query!(
        r#"SELECT id, workspace_id, name, amount, next_billing_date
           FROM subscriptions
           WHERE is_active = true AND next_billing_date <= $1"#,
        limite
    )
    .fetch_all(pool)
    .await?;

    for fila in filas {
        if ya_notificado(pool, "subscription_due", fila.id, true).await? {
            continue;
        }
        let body = format!(
            "Se cobrará {:.2} el {}.",
            fila.amount, fila.next_billing_date
        );
        crear_notificacion(
            pool,
            fila.workspace_id,
            "subscription_due",
            &format!("Próximo a vencer: {}", fila.name),
            &body,
            fila.id,
        )
        .await?;
    }
    Ok(())
}

/// Presupuestos del mes en curso al 80% o 100% de su límite — misma
/// agregación que `accounting::presupuestos::estado`, sin filtrar por
/// workspace. Ambos umbrales se evalúan y deduplican por separado.
async fn revisar_presupuestos(pool: &PgPool) -> Result<(), sqlx::Error> {
    let mes = primer_dia_del_mes(Utc::now().date_naive());

    let filas = sqlx::query!(
        r#"SELECT b.id, b.workspace_id, c.name AS category_name, b.limit_amount,
                  COALESCE(SUM(t.amount), 0) AS "spent!"
           FROM budgets b
           JOIN categories c ON c.id = b.category_id
           LEFT JOIN transactions t
               ON t.category_id = b.category_id
              AND t.workspace_id = b.workspace_id
              AND t.type = 'expense'
              AND t.is_active = true
              AND date_trunc('month', t.date) = b.month
           WHERE b.month = $1
           GROUP BY b.id, c.name, b.limit_amount"#,
        mes
    )
    .fetch_all(pool)
    .await?;

    for fila in filas {
        let porcentaje = fila.spent * Decimal::from(100) / fila.limit_amount;
        let umbral = if porcentaje >= Decimal::from(100) {
            Some(("budget_100", "Presupuesto alcanzado"))
        } else if porcentaje >= Decimal::from(80) {
            Some(("budget_80", "Presupuesto en 80%"))
        } else {
            None
        };

        let Some((tipo, titulo)) = umbral else {
            continue;
        };
        if ya_notificado(pool, tipo, fila.id, false).await? {
            continue;
        }

        let body = format!(
            "{}: llevas {:.2} de {:.2} ({:.0}%).",
            fila.category_name, fila.spent, fila.limit_amount, porcentaje
        );
        crear_notificacion(
            pool,
            fila.workspace_id,
            tipo,
            &format!("{titulo}: {}", fila.category_name),
            &body,
            fila.id,
        )
        .await?;
    }
    Ok(())
}
