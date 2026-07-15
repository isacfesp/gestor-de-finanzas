// =====================================================================
// graficos.rs — Gráfica de tendencia (línea) y de flujo de dinero
// (pastel), renderizadas en el servidor con `charts-rs` y devueltas
// como SVG de texto — el frontend lo inyecta con `inner_html`, sin
// pasar por <img>/canvas/WASM.
//
// `tema` (claro/oscuro) elige la paleta en el momento de generar el
// SVG: un SVG ya armado no puede reaccionar a un cambio de tema hecho
// después en el navegador, así que el frontend vuelve a pedir el
// gráfico cuando el usuario alterna el tema (ver `theme.rs`).
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
};
use charts_rs::{Color, LineChart, PieChart, Series};
use chrono::{Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use sqlx::PgPool;
use uuid::Uuid;

use crate::analytics::comun::resolver_filtro_usuario;
use crate::analytics::models::{FiltroFlujoPastel, FiltroTendencia, GraficoSvg};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

/// Paleta categórica fija (skill dataviz, `references/palette.md`) —
/// mismos valores hex que `--series-1..8` en
/// `frontend/styles/tailwind.css`. Duplicada aquí porque el SVG se
/// genera en el backend, que no puede leer variables CSS del cliente.
/// Orden fijo, nunca ciclada; una 9ª categoría se pliega en "Otros"
/// (ver `categoria_color`).
const SERIES_OSCURO: [&str; 8] = [
    "#3987e5", "#199e70", "#c98500", "#008300", "#9085e9", "#e66767", "#d55181", "#d95926",
];
const SERIES_CLARO: [&str; 8] = [
    "#2a78d6", "#1baf7a", "#eda100", "#008300", "#4a3aa7", "#e34948", "#e87ba4", "#eb6834",
];
const OTROS_OSCURO: &str = "#5b6b8c";
const OTROS_CLARO: &str = "#94a3b8";

struct Paleta {
    series: [&'static str; 8],
    otros: &'static str,
    positivo: &'static str,
    negativo: &'static str,
    texto: &'static str,
    linea: &'static str,
}

fn paleta(tema: Option<&str>) -> Paleta {
    match tema {
        Some("light") => Paleta {
            series: SERIES_CLARO,
            otros: OTROS_CLARO,
            positivo: "#16a34a",
            negativo: "#e11d48",
            texto: "#0c1b3a",
            linea: "#dbe3f0",
        },
        _ => Paleta {
            series: SERIES_OSCURO,
            otros: OTROS_OSCURO,
            positivo: "#34d399",
            negativo: "#fb7185",
            texto: "#eaf0ff",
            linea: "#243057",
        },
    }
}

/// La 9ª categoría en adelante (ordenadas por monto descendente, ya
/// como las devuelve la consulta) cae en "Otros" — mismo criterio que
/// la barra apilada de `distribucion.rs` en el frontend.
fn categoria_color(paleta: &Paleta, indice: usize) -> Color {
    match paleta.series.get(indice) {
        Some(hex) => Color::from(*hex),
        None => Color::from(paleta.otros),
    }
}

fn decimal_a_f32(valor: Decimal) -> f32 {
    valor.to_f64().unwrap_or(0.0) as f32
}

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

/// GET /workspaces/:workspace_id/analytics/charts/tendencia?tema=&granularidad=
///
/// Ingresos/egresos según la vista elegida (`granularidad`): "semana"
/// muestra la semana en curso día por día, "mes" el mes en curso
/// semana por semana, y "año" (por defecto "mes") los últimos 12
/// meses, mes por mes — no existe un endpoint de serie temporal
/// reutilizable (`flujo_caja` en `metricas.rs` solo agrega un rango a
/// un único total), así que esta consulta agrupa aparte.
pub async fn tendencia(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroTendencia>,
) -> Result<Json<GraficoSvg>, AppError> {
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
    // la escala temporal uniforme y las barras/puntos "se acercan").
    let monto_de = |periodo: NaiveDate, elegir: fn(&Decimal, &Decimal) -> Decimal| {
        filas
            .iter()
            .find(|f| f.periodo == periodo)
            .map(|f| decimal_a_f32(elegir(&f.income, &f.expense)))
            .unwrap_or(0.0)
    };
    let ingresos: Vec<f32> = grilla.iter().map(|m| monto_de(*m, |i, _| *i)).collect();
    let egresos: Vec<f32> = grilla.iter().map(|m| monto_de(*m, |_, e| *e)).collect();
    let etiquetas: Vec<String> = grilla.iter().map(|m| etiqueta_punto(*m, vista)).collect();

    // Sin esto, cuando no hay movimientos en todo el rango (workspace
    // nuevo/de prueba) `charts-rs` calcula el eje Y como min == max ==
    // 0.0 y termina dividiendo 0.0/0.0 = NaN para cada punto — mismo
    // criterio de "cortar antes de dividir" que ya usa
    // `metricas::tasa_ahorro` para `total_income == 0`.
    if ingresos.iter().chain(egresos.iter()).all(|v| *v == 0.0) {
        return Ok(Json(GraficoSvg { svg: String::new() }));
    }

    let paleta = paleta(filtro.tema.as_deref());
    let mut grafico = LineChart::new(
        vec![
            Series::new("Ingresos".to_string(), ingresos),
            Series::new("Egresos".to_string(), egresos),
        ],
        etiquetas,
    );
    grafico.width = 600.0;
    grafico.height = 280.0;
    grafico.background_color = Color::transparent();
    grafico.font_family = "Plus Jakarta Sans".to_string();
    grafico.series_colors = vec![Color::from(paleta.positivo), Color::from(paleta.negativo)];
    grafico.series_smooth = true;
    grafico.legend_font_color = Color::from(paleta.texto);
    grafico.x_axis_font_color = Color::from(paleta.texto);
    grafico.x_axis_stroke_color = Color::from(paleta.linea);
    grafico.grid_stroke_color = Color::from(paleta.linea);

    let svg = grafico
        .svg()
        .map_err(|e| AppError::Interno(format!("charts-rs (tendencia): {e}")))?;

    Ok(Json(GraficoSvg { svg }))
}

/// GET /workspaces/:workspace_id/analytics/charts/flujo-pastel?desde=&hasta=&tema=
///
/// Ingresos Y gastos por categoría en el rango pedido, en un solo
/// pastel — cada rebanada es una categoría de un tipo (`"Ingreso ·
/// Sueldo"`, `"Gasto · Comida"`), con el porcentaje calculado sobre el
/// total combinado (`PieChart` reparte proporciones sobre la suma de
/// todas las series que recibe). Se agrupa por `(type, category_name)`
/// y no solo por `category_name` para no mezclar el bucket "Sin
/// categoría" de ingreso con el de gasto.
pub async fn flujo_pastel(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(filtro): Query<FiltroFlujoPastel>,
) -> Result<Json<GraficoSvg>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    let filtro_usuario = resolver_filtro_usuario(&usuario, filtro.user_id);

    let filas = sqlx::query!(
        r#"SELECT t.type AS "tipo!", COALESCE(c.name, 'Sin categoría') AS "category_name!",
                  SUM(t.amount) AS "amount!"
           FROM transactions t
           LEFT JOIN categories c ON c.id = t.category_id
           WHERE t.workspace_id = $1 AND t.is_active = true
             AND ($2::date IS NULL OR t.date >= $2)
             AND ($3::date IS NULL OR t.date <= $3)
             AND ($4::uuid IS NULL OR t.created_by = $4)
           GROUP BY t.type, c.name
           ORDER BY 3 DESC"#,
        workspace_id,
        filtro.desde,
        filtro.hasta,
        filtro_usuario
    )
    .fetch_all(&pool)
    .await?;

    // `PieChart` con lista vacía ya degrada bien hoy (no dibuja
    // ninguna rebanada), pero no depender de ese detalle no
    // documentado de una dependencia externa — cortar acá es más
    // robusto y barato.
    if filas.is_empty() {
        return Ok(Json(GraficoSvg { svg: String::new() }));
    }

    let paleta = paleta(filtro.tema.as_deref());
    let series_list: Vec<Series> = filas
        .iter()
        .map(|f| {
            let prefijo = if f.tipo == "income" {
                "Ingreso"
            } else {
                "Gasto"
            };
            Series::new(
                format!("{prefijo} · {}", f.category_name),
                vec![decimal_a_f32(f.amount)],
            )
        })
        .collect();
    let colores: Vec<Color> = (0..series_list.len())
        .map(|i| categoria_color(&paleta, i))
        .collect();

    let mut grafico = PieChart::new(series_list);
    grafico.width = 320.0;
    grafico.height = 320.0;
    grafico.background_color = Color::transparent();
    grafico.font_family = "Plus Jakarta Sans".to_string();
    grafico.series_colors = colores;
    // "Pastel" pedido explícitamente, no dona ni rosa (nightingale) —
    // son los defaults de `PieChart::new`, hay que apagarlos a mano.
    grafico.inner_radius = 0.0;
    grafico.rose_type = Some(false);
    grafico.legend_show = Some(true);
    grafico.legend_font_color = Color::from(paleta.texto);
    grafico.title_font_color = Color::from(paleta.texto);

    let svg = grafico
        .svg()
        .map_err(|e| AppError::Interno(format!("charts-rs (flujo-pastel): {e}")))?;

    Ok(Json(GraficoSvg { svg }))
}
