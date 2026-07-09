//! Distribución de gastos por categoría: barra horizontal apilada
//! (part-to-whole) + leyenda — diseño validado con la skill `dataviz`
//! (nunca donut; paleta categórica fija de 8 colores; "Otros" para la
//! cola más allá de 8). Comparte el filtro desde/hasta con `Kpis`.

use chrono::NaiveDate;
use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::api::analytics::{self, DistribucionGasto};
use crate::auth::{token_vigente, use_auth};

const COLORES: [&str; 8] = [
    "var(--series-1)",
    "var(--series-2)",
    "var(--series-3)",
    "var(--series-4)",
    "var(--series-5)",
    "var(--series-6)",
    "var(--series-7)",
    "var(--series-8)",
];

/// El color de una categoría depende de su nombre, no de su posición en
/// la lista — así no cambia de color si el filtro de fecha reordena o
/// hace desaparecer categorías (la identidad manda, nunca el rango).
fn color_para(nombre: &str) -> &'static str {
    let hash = nombre
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    COLORES[(hash % 8) as usize]
}

/// Más de 8 categorías no caben en la paleta categórica: las de menor
/// monto (el backend ya las ordena desc) se pliegan en un bucket
/// "Otros" en vez de generar más colores.
fn con_bucket_otros(
    gastos: Vec<DistribucionGasto>,
) -> Vec<(String, Decimal, Decimal, &'static str)> {
    if gastos.len() <= 8 {
        return gastos
            .into_iter()
            .map(|g| {
                let color = color_para(&g.category_name);
                (g.category_name, g.amount, g.percentage, color)
            })
            .collect();
    }

    let (principales, resto) = gastos.split_at(7);
    let mut filas: Vec<_> = principales
        .iter()
        .map(|g| {
            (
                g.category_name.clone(),
                g.amount,
                g.percentage,
                color_para(&g.category_name),
            )
        })
        .collect();

    let monto_otros = resto.iter().fold(Decimal::ZERO, |acc, g| acc + g.amount);
    let pct_otros = resto
        .iter()
        .fold(Decimal::ZERO, |acc, g| acc + g.percentage);
    filas.push(("Otros".to_string(), monto_otros, pct_otros, "var(--muted)"));
    filas
}

#[component]
pub fn Distribucion(
    workspace_id: Uuid,
    desde: RwSignal<String>,
    hasta: RwSignal<String>,
) -> impl IntoView {
    let auth = use_auth();

    let gastos = LocalResource::new(move || {
        let desde_txt = desde.get();
        let hasta_txt = hasta.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let desde: Option<NaiveDate> = desde_txt.parse().ok();
            let hasta: Option<NaiveDate> = hasta_txt.parse().ok();
            analytics::distribucion_gastos(workspace_id, desde, hasta, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h2>"Distribución de gastos"</h2>
            </div>
            {move || match gastos.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"No hay gastos en este período."</p> }.into_any()
                }
                Some(Ok(lista)) => {
                    let filas = con_bucket_otros(lista);
                    view! {
                        <div>
                            <div class="series-track">
                                {filas
                                    .iter()
                                    .map(|(nombre, monto, pct, color)| {
                                        let ancho_f64: f64 = pct.to_string().parse().unwrap_or(0.0);
                                        let titulo = format!("{nombre}: {monto:.2} ({pct:.0}%)");
                                        view! {
                                            <div
                                                class="series-segment"
                                                title=titulo
                                                style=format!("width:{ancho_f64}%; background:{color};")
                                            ></div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                            <div class="series-legend">
                                {filas
                                    .into_iter()
                                    .map(|(nombre, monto, pct, color)| {
                                        view! {
                                            <div class="series-legend-item">
                                                <span class="series-swatch" style=format!("background:{color};")></span>
                                                <span>{nombre}</span>
                                                <span class="series-amount">{format!("{:.2} ({:.0}%)", monto, pct)}</span>
                                            </div>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        </div>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}
