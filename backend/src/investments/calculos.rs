// =====================================================================
// calculos.rs — Matemática financiera del módulo investments.
//
// Funciones puras (sin base de datos) para que tanto el simulador como
// la proyección de una inversión ya registrada compartan exactamente
// la misma fórmula.
// =====================================================================

use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;

use crate::errores::AppError;
use crate::investments::models::DesgloseRendimiento;

const DIAS_POR_ANIO: i64 = 365;

/// Tasa de ISR sugerida (porcentaje, no fracción) para prellenar el
/// formulario de alta y como default del simulador. Cada inversión
/// real guarda su propia tasa en `isr_annual_rate` — esta constante ya
/// no aplica a todas por igual, es solo un punto de partida razonable
/// (la fija cada año la Ley de Ingresos de la Federación y cambia de
/// un ejercicio fiscal a otro).
pub fn tasa_retencion_isr_anual() -> Decimal {
    Decimal::new(50, 2) // 0.50 %
}

pub fn validar_tipo_interes(tipo: &str) -> Result<(), AppError> {
    if tipo == "simple" || tipo == "compound" {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El tipo de interés debe ser 'simple' o 'compound'".to_string(),
        ))
    }
}

pub fn validar_principal(monto: Decimal) -> Result<(), AppError> {
    if monto > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El capital debe ser mayor a cero".to_string(),
        ))
    }
}

pub fn validar_tasa(tasa: Decimal) -> Result<(), AppError> {
    if tasa > Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "La tasa GAT anual debe ser mayor a cero".to_string(),
        ))
    }
}

/// A diferencia de la tasa GAT, el ISR sí puede ser legítimamente 0
/// (algún régimen exento) — solo se rechaza un valor negativo.
pub fn validar_tasa_isr(tasa: Decimal) -> Result<(), AppError> {
    if tasa >= Decimal::ZERO {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "La tasa de ISR no puede ser negativa".to_string(),
        ))
    }
}

pub fn validar_plazo(dias: i32) -> Result<(), AppError> {
    if dias > 0 {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El plazo en días debe ser mayor a cero".to_string(),
        ))
    }
}

/// Interés generado por el capital durante `term_days`, sin ISR.
///
/// - Simple: interés lineal sobre el capital original.
/// - Compuesto: capitalización diaria (`checked_powi` hace exponenciación
///   por cuadrados repetidos, así que un plazo de años no cuesta más que
///   uno de días).
pub(super) fn rendimiento_bruto(
    principal: Decimal,
    gat_annual_rate: Decimal,
    term_days: i32,
    interest_type: &str,
) -> Result<Decimal, AppError> {
    let tasa_decimal = gat_annual_rate / Decimal::ONE_HUNDRED;
    let dias = Decimal::from(term_days);

    match interest_type {
        "simple" => Ok(principal * tasa_decimal * dias / Decimal::from(DIAS_POR_ANIO)),
        "compound" => {
            let factor_diario = Decimal::ONE + tasa_decimal / Decimal::from(DIAS_POR_ANIO);
            let factor_total = factor_diario
                .checked_powi(term_days as i64)
                .ok_or_else(|| {
                    AppError::NoProcesable(
                    "El plazo o la tasa son demasiado grandes para calcular el interés compuesto"
                        .to_string(),
                )
                })?;
            Ok(principal * factor_total - principal)
        }
        _ => unreachable!("tipo ya validado por validar_tipo_interes"),
    }
}

/// ISR retenido sobre el capital que originó los intereses, prorrateado
/// por los días reales de la inversión (no hay monto exento vigente).
/// `isr_annual_rate` es la tasa propia de cada inversión (o la del
/// simulador), ya no una constante global.
pub(super) fn isr_retenido(
    principal: Decimal,
    term_days: i32,
    isr_annual_rate: Decimal,
) -> Decimal {
    let tasa_decimal = isr_annual_rate / Decimal::ONE_HUNDRED;
    principal * tasa_decimal * Decimal::from(term_days) / Decimal::from(DIAS_POR_ANIO)
}

/// Arma el desglose completo (bruto, ISR, neto, monto al vencimiento)
/// a partir de los datos financieros de una inversión, sea real o
/// simulada.
pub fn calcular_desglose(
    principal: Decimal,
    gat_annual_rate: Decimal,
    interest_type: &str,
    term_days: i32,
    isr_annual_rate: Decimal,
) -> Result<DesgloseRendimiento, AppError> {
    let bruto = rendimiento_bruto(principal, gat_annual_rate, term_days, interest_type)?;
    let isr = isr_retenido(principal, term_days, isr_annual_rate);
    let neto = bruto - isr;

    Ok(DesgloseRendimiento {
        principal,
        gat_annual_rate,
        interest_type: interest_type.to_string(),
        term_days,
        gross_yield: bruto,
        isr_amount: isr,
        net_yield: neto,
        maturity_amount: principal + neto,
    })
}
