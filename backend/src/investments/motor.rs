// =====================================================================
// motor.rs — Ciclo en background: calcula a diario el rendimiento y el
// ISR generado por cada inversión activa de TODOS los workspaces, y lo
// persiste en investment_accruals. No es un handler HTTP: se lanza una
// sola vez desde main() con tokio::spawn, en un ciclo separado del de
// reminders::motor (responsabilidades distintas — un fallo en el
// cálculo financiero no debe frenar las notificaciones ni viceversa).
// =====================================================================

use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

use crate::investments::calculos::{isr_retenido, rendimiento_bruto};

/// La spec pide un ciclo "diario"; se aproxima con un intervalo fijo
/// desde el arranque, mismo criterio que reminders::motor.
const INTERVALO_CICLO: Duration = Duration::from_secs(60 * 60 * 24);

pub async fn ejecutar_ciclo_periodico(pool: PgPool) {
    loop {
        if let Err(e) = ejecutar_ciclo(&pool).await {
            eprintln!("ERROR en ciclo de accrual de inversiones: {e}");
        }
        tokio::time::sleep(INTERVALO_CICLO).await;
    }
}

struct InversionActiva {
    id: Uuid,
    principal: Decimal,
    gat_annual_rate: Decimal,
    isr_annual_rate: Decimal,
    interest_type: String,
    start_date: NaiveDate,
    end_date: NaiveDate,
}

async fn ejecutar_ciclo(pool: &PgPool) -> Result<(), sqlx::Error> {
    let hoy = Utc::now().date_naive();

    let inversiones = sqlx::query_as!(
        InversionActiva,
        r#"SELECT id, principal, gat_annual_rate, isr_annual_rate, interest_type,
                  start_date, end_date
           FROM investments
           WHERE is_active = true AND start_date <= $1"#,
        hoy
    )
    .fetch_all(pool)
    .await?;

    for inversion in inversiones {
        if let Err(e) = procesar_inversion(pool, &inversion, hoy).await {
            eprintln!(
                "ERROR calculando accrual de la inversión {}: {e}",
                inversion.id
            );
        }
    }

    Ok(())
}

/// Calcula y guarda el accrual de cada día pendiente de una inversión,
/// desde el último ya calculado hasta `hoy` (o `end_date`, lo que sea
/// menor). El día marginal se obtiene como la diferencia entre el
/// rendimiento/ISR acumulados en `dias` vs `dias - 1` — correcto tanto
/// para interés simple como compuesto, sin llevar un acumulado aparte.
async fn procesar_inversion(
    pool: &PgPool,
    inversion: &InversionActiva,
    hoy: NaiveDate,
) -> Result<(), sqlx::Error> {
    let ultima_fecha = sqlx::query_scalar!(
        "SELECT MAX(accrual_date) FROM investment_accruals WHERE investment_id = $1",
        inversion.id
    )
    .fetch_one(pool)
    .await?
    .unwrap_or_else(|| inversion.start_date - chrono::Duration::days(1));

    let fecha_limite = hoy.min(inversion.end_date);
    if ultima_fecha >= fecha_limite {
        return Ok(()); // ya está al día — reinicio del proceso en el mismo día
    }

    let mut fecha = ultima_fecha + chrono::Duration::days(1);
    while fecha <= fecha_limite {
        let dias_transcurridos = (fecha - inversion.start_date).num_days() as i32 + 1;
        let dias_previos = dias_transcurridos - 1;

        let bruto_hasta_hoy = rendimiento_bruto(
            inversion.principal,
            inversion.gat_annual_rate,
            dias_transcurridos,
            &inversion.interest_type,
        );
        let bruto_hasta_ayer = rendimiento_bruto(
            inversion.principal,
            inversion.gat_annual_rate,
            dias_previos,
            &inversion.interest_type,
        );

        let (Ok(bruto_hasta_hoy), Ok(bruto_hasta_ayer)) = (bruto_hasta_hoy, bruto_hasta_ayer)
        else {
            // Plazo/tasa demasiado grandes para el interés compuesto:
            // se salta este día y se sigue con el resto de inversiones.
            eprintln!(
                "AVISO: no se pudo calcular el accrual del {fecha} para la inversión {}",
                inversion.id
            );
            fecha += chrono::Duration::days(1);
            continue;
        };

        let isr_hasta_hoy = isr_retenido(
            inversion.principal,
            dias_transcurridos,
            inversion.isr_annual_rate,
        );
        let isr_hasta_ayer =
            isr_retenido(inversion.principal, dias_previos, inversion.isr_annual_rate);

        let gross_yield = bruto_hasta_hoy - bruto_hasta_ayer;
        let isr_amount = isr_hasta_hoy - isr_hasta_ayer;
        let net_yield = gross_yield - isr_amount;

        sqlx::query!(
            r#"INSERT INTO investment_accruals
                   (investment_id, accrual_date, gross_yield, isr_amount, net_yield)
               VALUES ($1, $2, $3, $4, $5)
               ON CONFLICT (investment_id, accrual_date) DO NOTHING"#,
            inversion.id,
            fecha,
            gross_yield,
            isr_amount,
            net_yield
        )
        .execute(pool)
        .await?;

        fecha += chrono::Duration::days(1);
    }

    Ok(())
}
