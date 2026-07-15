// =====================================================================
// graficos.rs — Datos de la gráfica de tendencia (línea) y de flujo de
// dinero por categoría (pastel). Antes esto se renderizaba en el
// servidor con `charts-rs` y se devolvía un SVG de texto; ahora se
// devuelven los números y el frontend dibuja e interactúa con la
// gráfica él mismo (hover/tap para ver el monto exacto, soporte de
// saldo negativo) — un SVG ya armado no puede reaccionar a eventos de
// puntero ni recolorearse solo, así que había que elegir entre eso o
// reescribir el SVG en el servidor por cada frame de interacción.
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
    DatosFlujoPastel, DatosTendencia, FiltroFlujoPastel, FiltroTendencia, PuntoTendencia,
    RebanadaPastel,
};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

fn primer_dia_del_mes(fecha: NaiveDate) -> NaiveDate {
    fecha
        .with_day(1)
        .expect("el día 1 siempre es válido en cualquier mes")
}

/// Suma (o resta, con `meses` negativo) meses calendario a una fecha,
/// preservando el día 1 — solo se usa sobre fechas ya truncadas al
/// primer día del mes, así que siempre da un resultado válido.
fn sumar_meses(fecha: NaiveDate, meses: i64) -> NaiveDate {
    let total = fecha.year() as i64 * 12 + (fecha.month() as i64 - 1) + meses;
    let anio = total.div_euclid(12) as i32;
    let mes = total.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(anio, mes, 1).expect("día 1 de un mes calendario siempre es válido")
}

const MESES_ABREV: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];

fn etiqueta_mes(fecha: NaiveDate) -> String {
    format!("{} {}", MESES_ABREV[fecha.month0() as usize], fecha.year())
}

/// Vista de la gráfica de tendencia — a diferencia de un diseño
/// genérico "granularidad + cantidad de períodos", cada vista fija
/// tanto el rango de fechas como la unidad de agrupación, porque así
/// lo pidió el negocio: "semana" es la semana en curso día por día,
/// "mes" es el mes en curso semana por semana, y "año" son los últimos
/// 12 meses, mes por mes. Un `match` cerrado a estas 3 variantes es lo
/// que hace seguro pasar `date_trunc_pg(v)` como parámetro de
/// `date_trunc` en SQL: nunca puede llegar un string arbitrario del
/// cliente, solo uno de estos 3 literales fijos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vista {
    Semana,
    Mes,
    Anio,
}

fn parsear_vista(valor: Option<&str>) -> Vista {
    match valor {
        Some("semana") => Vista::Semana,
        Some("año") | Some("anio") => Vista::Anio,
        _ => Vista::Mes,
    }
}

fn date_trunc_pg(v: Vista) -> &'static str {
    match v {
        Vista::Semana => "day",
        Vista::Mes => "week",
        Vista::Anio => "month",
    }
}

/// Lunes de la semana calendario (ISO) que contiene `fecha`.
fn lunes_de_semana(fecha: NaiveDate) -> NaiveDate {
    fecha - chrono::Duration::days(fecha.weekday().num_days_from_monday() as i64)
}

fn ultimo_dia_del_mes(primer_dia: NaiveDate) -> NaiveDate {
    sumar_meses(primer_dia, 1) - chrono::Duration::days(1)
}

const DIAS_ABREV: [&str; 7] = ["lun", "mar", "mié", "jue", "vie", "sáb", "dom"];

fn etiqueta_dia(fecha: NaiveDate) -> String {
    format!(
        "{} {}",
        DIAS_ABREV[fecha.weekday().num_days_from_monday() as usize],
        fecha.day()
    )
}

fn etiqueta_semana(fecha: NaiveDate) -> String {
    fecha.format("%d/%m").to_string()
}

fn etiqueta_punto(fecha: NaiveDate, v: Vista) -> String {
    match v {
        Vista::Semana => etiqueta_dia(fecha),
        Vista::Mes => etiqueta_semana(fecha),
        Vista::Anio => etiqueta_mes(fecha),
    }
}

/// Rango de fechas a consultar (inclusive en ambos extremos) y grilla
/// completa de puntos del eje X, según la vista elegida.
fn rango_y_grilla(v: Vista, hoy: NaiveDate) -> (NaiveDate, NaiveDate, Vec<NaiveDate>) {
    match v {
        Vista::Semana => {
            let lunes = lunes_de_semana(hoy);
            let domingo = lunes + chrono::Duration::days(6);
            let grilla = (0..7).map(|i| lunes + chrono::Duration::days(i)).collect();
            (lunes, domingo, grilla)
        }
        Vista::Mes => {
            let primer_dia = primer_dia_del_mes(hoy);
            let ultimo_dia = ultimo_dia_del_mes(primer_dia);
            // Grilla por semana ISO: la primera/última semana del mes
            // pueden empezar en el mes anterior/terminar en el
            // siguiente — se etiquetan igual, por su lunes.
            let primera_semana = lunes_de_semana(primer_dia);
            let ultima_semana = lunes_de_semana(ultimo_dia);
            let cantidad_semanas = (ultima_semana - primera_semana).num_weeks() + 1;
            let grilla = (0..cantidad_semanas)
                .map(|i| primera_semana + chrono::Duration::weeks(i))
                .collect();
            (primer_dia, ultimo_dia, grilla)
        }
        Vista::Anio => {
            let fin = primer_dia_del_mes(hoy);
            let inicio = sumar_meses(fin, -11);
            let grilla = (0..12).map(|i| sumar_meses(inicio, i)).collect();
            (inicio, ultimo_dia_del_mes(fin), grilla)
        }
    }
}

/// GET /workspaces/:workspace_id/analytics/charts/tendencia?granularidad=
///
/// Ingresos/egresos/saldo neto según la vista elegida (`granularidad`):
/// "semana" muestra la semana en curso día por día, "mes" el mes en
/// curso semana por semana, y "año" (por defecto "mes") los últimos 12
/// meses, mes por mes — no existe un endpoint de serie temporal
/// reutilizable (`flujo_caja` en `metricas.rs` solo agrega un rango a
/// un único total), así que esta consulta agrupa aparte.
pub async fn tendencia(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroTendencia>,
) -> Result<Json<DatosTendencia>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let vista = parsear_vista(filtro.granularidad.as_deref());
    let (rango_inicio, rango_fin, grilla) = rango_y_grilla(vista, Utc::now().date_naive());

    let filas = sqlx::query!(
        r#"SELECT date_trunc($5::text, date)::date AS "periodo!",
               COALESCE(SUM(amount) FILTER (WHERE type = 'income'), 0) AS "income!",
               COALESCE(SUM(amount) FILTER (WHERE type = 'expense'), 0) AS "expense!"
           FROM transactions
           WHERE workspace_id = $1 AND is_active = true
             AND date >= $2 AND date <= $3
             AND ($4::uuid IS NULL OR created_by = $4)
           GROUP BY 1
           ORDER BY 1"#,
        workspace_id,
        rango_inicio,
        rango_fin,
        filtro_usuario,
        date_trunc_pg(vista)
    )
    .fetch_all(&pool)
    .await?;

    // Rejilla completa de puntos: aunque uno no tenga movimientos, el
    // eje X debe mostrarlo en 0, no saltárselo (si no, el eje pierde
    // la escala temporal uniforme y los puntos "se acercan").
    let fila_de = |periodo: NaiveDate| filas.iter().find(|f| f.periodo == periodo);

    let puntos: Vec<PuntoTendencia> = grilla
        .iter()
        .map(|periodo| {
            let (income, expense) = fila_de(*periodo)
                .map(|f| (f.income, f.expense))
                .unwrap_or((Decimal::ZERO, Decimal::ZERO));
            PuntoTendencia {
                etiqueta: etiqueta_punto(*periodo, vista),
                income,
                expense,
                net: income - expense,
            }
        })
        .collect();

    Ok(Json(DatosTendencia { puntos }))
}

/// % que representa `monto` sobre `total` — `0` si `total` es cero en
/// vez de dividir (evita el panic de `rust_decimal` al dividir por
/// cero cuando un tipo no tiene movimientos en el rango).
fn porcentaje(monto: Decimal, total: Decimal) -> Decimal {
    if total.is_zero() {
        Decimal::ZERO
    } else {
        (monto / total) * Decimal::from(100)
    }
}

/// GET /workspaces/:workspace_id/analytics/charts/flujo-pastel?desde=&hasta=
///
/// Ingresos y gastos por categoría en el rango pedido, como dos listas
/// separadas — antes era un solo pastel mezclando ambos tipos, con el
/// porcentaje de cada rebanada calculado sobre el total combinado
/// (ingreso + gasto), así que ninguna de las dos mitades sumaba 100%
/// por separado. Cada lista de acá trae su `percentage` relativo solo
/// a su propio total.
pub async fn flujo_pastel(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroFlujoPastel>,
) -> Result<Json<DatosFlujoPastel>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let filas = sqlx::query!(
        r#"SELECT t.type AS "tipo!", c.id AS "category_id?",
                  COALESCE(c.name, 'Sin categoría') AS "category_name!",
                  SUM(t.amount) AS "amount!"
           FROM transactions t
           LEFT JOIN categories c ON c.id = t.category_id
           WHERE t.workspace_id = $1 AND t.is_active = true
             AND ($2::date IS NULL OR t.date >= $2)
             AND ($3::date IS NULL OR t.date <= $3)
             AND ($4::uuid IS NULL OR t.created_by = $4)
           GROUP BY t.type, c.id, c.name
           ORDER BY 4 DESC"#,
        workspace_id,
        filtro.desde,
        filtro.hasta,
        filtro_usuario
    )
    .fetch_all(&pool)
    .await?;

    let total_de = |tipo: &str| -> Decimal {
        filas
            .iter()
            .filter(|f| f.tipo == tipo)
            .map(|f| f.amount)
            .sum()
    };
    let total_ingresos = total_de("income");
    let total_gastos = total_de("expense");

    let rebanadas_de = |tipo: &str, total: Decimal| -> Vec<RebanadaPastel> {
        filas
            .iter()
            .filter(|f| f.tipo == tipo)
            .map(|f| RebanadaPastel {
                category_id: f.category_id,
                category_name: f.category_name.clone(),
                amount: f.amount,
                percentage: porcentaje(f.amount, total),
            })
            .collect()
    };

    Ok(Json(DatosFlujoPastel {
        ingresos: rebanadas_de("income", total_ingresos),
        gastos: rebanadas_de("expense", total_gastos),
    }))
}
