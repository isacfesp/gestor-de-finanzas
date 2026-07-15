//! Tendencia de ingresos/egresos/saldo neto — gráfica de línea, con 3
//! vistas elegibles por el usuario: "Semanal" (la semana en curso, día
//! por día), "Mensual" (el mes en curso, semana por semana) y "Anual"
//! (los últimos 12 meses, mes por mes). El rango no es configurable
//! por separado de la vista — cada una fija ambos (ver
//! `analytics::graficos::rango_y_grilla` en el backend).
//!
//! Antes esto era un SVG ya armado por `charts-rs` en el servidor,
//! inyectado con `inner_html` — no podía reaccionar a un puntero ni
//! mostrar saldo negativo (solo mandaba ingresos/egresos, siempre
//! positivos). Ahora el backend manda los números y acá se dibuja el
//! `<svg>` a mano: cada punto tiene su propia zona de toque para
//! mostrar el monto exacto al pasar el cursor o al presionar (mobile),
//! y se agrega la serie "Neto" (ingresos − egresos), que si cae bajo
//! cero se ve claramente bajo la línea base — significa que ese
//! período se gastó más de lo que entró.

use leptos::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

use crate::api::analytics::{self, PuntoTendencia};
use crate::auth::{token_vigente, use_auth};

fn titulo_vista(granularidad: &str) -> &'static str {
    match granularidad {
        "semana" => "Tendencia (esta semana)",
        "año" => "Tendencia (últimos 12 meses)",
        _ => "Tendencia (este mes)",
    }
}

// Lienzo lógico del <svg> — se escala solo al ancho real vía
// `viewBox` + `width:100%`, así que estas constantes son "unidades",
// no pixeles de pantalla.
const ANCHO: f64 = 600.0;
const ALTO: f64 = 300.0;
const MARGEN_IZQ: f64 = 8.0;
const MARGEN_DER: f64 = 8.0;
// Franja superior reservada para el tooltip: así nunca se superpone
// con las líneas de datos, sin importar en qué punto esté el activo.
const MARGEN_ARRIBA: f64 = 46.0;
const MARGEN_ABAJO: f64 = 26.0;

fn decimal_f64(v: Decimal) -> f64 {
    v.to_f64().unwrap_or(0.0)
}

/// Posición horizontal del punto `indice` de `total`, repartidos en
/// partes iguales sobre el ancho graficable.
fn x_de(indice: usize, total: usize) -> f64 {
    let ancho_plot = ANCHO - MARGEN_IZQ - MARGEN_DER;
    if total <= 1 {
        MARGEN_IZQ + ancho_plot / 2.0
    } else {
        MARGEN_IZQ + indice as f64 / (total - 1) as f64 * ancho_plot
    }
}

/// Posición vertical de `valor` dentro del rango `[min_val, max_val]`
/// — más grande el valor, más arriba (Y crece hacia abajo en SVG).
fn y_de(valor: f64, min_val: f64, max_val: f64) -> f64 {
    let alto_plot = ALTO - MARGEN_ARRIBA - MARGEN_ABAJO;
    let rango = (max_val - min_val).max(0.01);
    MARGEN_ARRIBA + (max_val - valor) / rango * alto_plot
}

/// Rango del eje Y para los 3 valores en cada punto (ingresos, egresos,
/// neto): siempre incluye el 0 (para que un neto negativo se vea bajo
/// la línea base, no recortado) y le agrega un 10% de colchón arriba y
/// abajo para que ningún punto quede pegado al borde del gráfico.
fn rango_y(valores: &[f64]) -> (f64, f64) {
    let min_bruto = valores.iter().cloned().fold(0.0_f64, f64::min);
    let max_bruto = valores.iter().cloned().fold(0.0_f64, f64::max);
    let colchon = (max_bruto - min_bruto).max(1.0) * 0.1;
    (min_bruto - colchon, max_bruto + colchon)
}

fn polilinea(coords: &[(f64, f64)]) -> String {
    coords
        .iter()
        .map(|(x, y)| format!("{x:.1},{y:.1}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn f(v: f64) -> String {
    format!("{v:.2}")
}

#[component]
pub fn Tendencia(workspace_id: Uuid, alcance: RwSignal<Option<Uuid>>) -> impl IntoView {
    let auth = use_auth();
    let granularidad = RwSignal::new("mes".to_string());
    let punto_activo = RwSignal::new(None::<usize>);

    let datos = LocalResource::new(move || {
        let alcance_actual = alcance.get();
        let granularidad_actual = granularidad.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            analytics::tendencia(workspace_id, alcance_actual, &granularidad_actual, &token)
                .await
                .map(|d| d.puntos)
                .map_err(|e| e.to_string())
        }
    });

    // Cambiar de vista redibuja el eje X entero — el índice que estaba
    // activo ya no corresponde a nada, mostrarlo confundiría más que
    // ayudar.
    Effect::new(move |_| {
        granularidad.get();
        punto_activo.set(None);
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h2>{move || titulo_vista(&granularidad.get())}</h2>
                <select
                    style="max-width:130px;"
                    prop:value=move || granularidad.get()
                    on:change=move |ev| granularidad.set(event_target_value(&ev))
                >
                    <option value="semana">"Semanal"</option>
                    <option value="mes">"Mensual"</option>
                    <option value="año">"Anual"</option>
                </select>
            </div>
            {move || match datos.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(puntos))
                    if puntos.iter().all(|p| p.income.is_zero() && p.expense.is_zero()) =>
                {
                    view! { <p class="text-soft">"Sin movimientos en este período."</p> }.into_any()
                }
                Some(Ok(puntos)) => {
                    view! { <GraficaTendencia puntos=puntos punto_activo=punto_activo/> }.into_any()
                }
            }}
        </section>
    }
}

#[component]
fn GraficaTendencia(
    puntos: Vec<PuntoTendencia>,
    punto_activo: RwSignal<Option<usize>>,
) -> impl IntoView {
    let n = puntos.len();
    let ingresos: Vec<f64> = puntos.iter().map(|p| decimal_f64(p.income)).collect();
    let egresos: Vec<f64> = puntos.iter().map(|p| decimal_f64(p.expense)).collect();
    let netos: Vec<f64> = puntos.iter().map(|p| decimal_f64(p.net)).collect();
    let (min_val, max_val) = rango_y(&[ingresos.clone(), egresos.clone(), netos.clone()].concat());

    let coords_de = |serie: &[f64]| -> Vec<(f64, f64)> {
        serie
            .iter()
            .enumerate()
            .map(|(i, v)| (x_de(i, n), y_de(*v, min_val, max_val)))
            .collect()
    };
    let coords_ingresos = coords_de(&ingresos);
    let coords_egresos = coords_de(&egresos);
    let coords_netos = coords_de(&netos);

    let hay_negativos = min_val < 0.0;
    let y_cero = y_de(0.0, min_val, max_val);
    let ancho_columna = if n > 1 {
        (ANCHO - MARGEN_IZQ - MARGEN_DER) / (n - 1) as f64
    } else {
        ANCHO - MARGEN_IZQ - MARGEN_DER
    };

    let etiquetas: Vec<String> = puntos.iter().map(|p| p.etiqueta.clone()).collect();

    view! {
        <div class="overflow-x-auto">
            <svg viewBox=format!("0 0 {ANCHO} {ALTO}") style="width:100%; max-width:640px; height:auto; display:block; margin:0 auto;">
                {hay_negativos
                    .then(|| {
                        view! {
                            <line
                                x1=f(MARGEN_IZQ)
                                x2=f(ANCHO - MARGEN_DER)
                                y1=f(y_cero)
                                y2=f(y_cero)
                                style="stroke:var(--faint); stroke-width:1; stroke-dasharray:3,3;"
                            ></line>
                        }
                    })}

                <polyline
                    points=polilinea(&coords_ingresos)
                    style="fill:none; stroke:var(--positive); stroke-width:2;"
                ></polyline>
                <polyline
                    points=polilinea(&coords_egresos)
                    style="fill:none; stroke:var(--negative); stroke-width:2;"
                ></polyline>
                <polyline
                    points=polilinea(&coords_netos)
                    style="fill:none; stroke:var(--accent); stroke-width:2.5;"
                ></polyline>

                {etiquetas
                    .iter()
                    .enumerate()
                    .map(|(i, etiqueta)| {
                        view! {
                            <text
                                x=f(x_de(i, n))
                                y=f(ALTO - 8.0)
                                style="text-anchor:middle; font-size:10px; fill:var(--faint);"
                            >
                                {etiqueta.clone()}
                            </text>
                        }
                    })
                    .collect_view()}

                // Zonas de toque invisibles, una franja vertical por
                // punto — más fáciles de acertar que el punto exacto,
                // sobre todo con el dedo.
                {(0..n)
                    .map(|i| {
                        let x0 = (x_de(i, n) - ancho_columna / 2.0).max(0.0);
                        view! {
                            <rect
                                x=f(x0)
                                y=f(MARGEN_ARRIBA)
                                width=f(ancho_columna)
                                height=f(ALTO - MARGEN_ARRIBA - MARGEN_ABAJO)
                                style="fill:transparent; cursor:pointer;"
                                on:mouseenter=move |_| punto_activo.set(Some(i))
                                on:mouseleave=move |_| punto_activo.set(None)
                                on:click=move |_| {
                                    punto_activo
                                        .update(|actual| {
                                            *actual = if *actual == Some(i) { None } else { Some(i) };
                                        })
                                }
                            ></rect>
                        }
                    })
                    .collect_view()}

                {move || {
                    punto_activo
                        .get()
                        .map(|i| {
                            let cx = x_de(i, n);
                            let punto = &puntos[i];
                            let caja_ancho = 172.0;
                            let caja_alto = 68.0;
                            let caja_x = (cx - caja_ancho / 2.0).clamp(2.0, ANCHO - caja_ancho - 2.0);
                            let caja_y = 4.0;
                            view! {
                                <g>
                                    <line
                                        x1=f(cx)
                                        x2=f(cx)
                                        y1=f(MARGEN_ARRIBA)
                                        y2=f(ALTO - MARGEN_ABAJO)
                                        style="stroke:var(--line); stroke-width:1;"
                                    ></line>
                                    <circle cx=f(cx) cy=f(y_de(decimal_f64(punto.income), min_val, max_val)) r="4" style="fill:var(--positive);"></circle>
                                    <circle cx=f(cx) cy=f(y_de(decimal_f64(punto.expense), min_val, max_val)) r="4" style="fill:var(--negative);"></circle>
                                    <circle cx=f(cx) cy=f(y_de(decimal_f64(punto.net), min_val, max_val)) r="4" style="fill:var(--accent);"></circle>

                                    <rect
                                        x=f(caja_x)
                                        y=f(caja_y)
                                        width=f(caja_ancho)
                                        height=f(caja_alto)
                                        rx="8"
                                        style="fill:var(--panel); stroke:var(--card-line); stroke-width:1;"
                                    ></rect>
                                    <text x=f(caja_x + 10.0) y=f(caja_y + 16.0) style="font-size:11px; font-weight:700; fill:var(--text);">
                                        {punto.etiqueta.clone()}
                                    </text>
                                    <text x=f(caja_x + 10.0) y=f(caja_y + 32.0) style="font-size:11px; fill:var(--positive);">
                                        {format!("Ingresos {:.2}", punto.income)}
                                    </text>
                                    <text x=f(caja_x + 10.0) y=f(caja_y + 46.0) style="font-size:11px; fill:var(--negative);">
                                        {format!("Egresos {:.2}", punto.expense)}
                                    </text>
                                    <text x=f(caja_x + 10.0) y=f(caja_y + 60.0) style="font-size:11px; font-weight:700; fill:var(--accent);">
                                        {format!("Neto {:.2}", punto.net)}
                                    </text>
                                </g>
                            }
                        })
                }}
            </svg>
        </div>

        <div class="mt-2 flex flex-wrap gap-4 text-[12px] text-muted">
            <span class="flex items-center gap-1.5">
                <span class="h-[8px] w-[8px] rounded-full" style="background:var(--positive);"></span>
                "Ingresos"
            </span>
            <span class="flex items-center gap-1.5">
                <span class="h-[8px] w-[8px] rounded-full" style="background:var(--negative);"></span>
                "Egresos"
            </span>
            <span class="flex items-center gap-1.5">
                <span class="h-[8px] w-[8px] rounded-full" style="background:var(--accent);"></span>
                "Neto"
            </span>
        </div>
    }
}
