//! Distribución de ingresos y gastos por categoría, como dos pasteles
//! separados (uno por tipo) — antes era un solo pastel mezclando
//! ambos tipos en un SVG ya armado por `charts-rs` en el servidor, con
//! el % de cada rebanada calculado sobre el total combinado (bug: las
//! rebanadas de "Gasto" no sumaban 100% entre ellas, sino relativo a
//! ingreso+gasto juntos). Ahora el backend manda el % de cada
//! categoría ya relativo a su propio tipo
//! (`analytics::graficos::flujo_pastel`) y acá se dibuja el `<svg>` a
//! mano, con zona de toque por rebanada para ver el monto exacto al
//! pasar el cursor o presionar (mobile). Comparte el filtro
//! desde/hasta con `Kpis`.

use chrono::NaiveDate;
use leptos::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use uuid::Uuid;

use crate::api::analytics::{self, RebanadaPastel};
use crate::auth::{token_vigente, use_auth};

fn decimal_f64(v: Decimal) -> f64 {
    v.to_f64().unwrap_or(0.0)
}

fn f(v: f64) -> String {
    format!("{v:.2}")
}

const SERIES: [&str; 8] = [
    "var(--series-1)",
    "var(--series-2)",
    "var(--series-3)",
    "var(--series-4)",
    "var(--series-5)",
    "var(--series-6)",
    "var(--series-7)",
    "var(--series-8)",
];

/// La 9ª categoría en adelante (ordenadas por monto descendente, como
/// ya las devuelve la consulta del backend) cae en "Otros" — mismo
/// criterio que la paleta categórica del resto de la app.
fn color_categoria(indice: usize) -> &'static str {
    SERIES.get(indice).copied().unwrap_or("var(--series-otros)")
}

fn punto_en_circulo(cx: f64, cy: f64, r: f64, angulo: f64) -> (f64, f64) {
    (cx + r * angulo.sin(), cy - r * angulo.cos())
}

/// Camino SVG de una rebanada de pastel, desde el ángulo `a0` hasta
/// `a1` (radianes, 0 = arriba, crece en sentido horario).
fn arco_pastel(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> String {
    let (x0, y0) = punto_en_circulo(cx, cy, r, a0);
    let (x1, y1) = punto_en_circulo(cx, cy, r, a1);
    let arco_grande = if (a1 - a0) > std::f64::consts::PI {
        1
    } else {
        0
    };
    format!(
        "M {cx:.2},{cy:.2} L {x0:.2},{y0:.2} A {r:.2},{r:.2} 0 {arco_grande} 1 {x1:.2},{y1:.2} Z"
    )
}

#[derive(Clone)]
struct RebanadaVisual {
    camino: Option<String>,
    centroide: (f64, f64),
    color: &'static str,
    nombre: String,
    monto: Decimal,
    porcentaje: Decimal,
}

/// Convierte cada rebanada en su geometría (ángulo acumulado → camino
/// SVG + punto central para la etiqueta de %). Una sola rebanada al
/// 100% no se puede dibujar como arco (el punto de inicio y fin
/// coinciden, `charts-rs` y cualquier renderer de arcos SVG lo pintan
/// vacío) — en ese caso `camino` queda en `None` y el caller dibuja un
/// `<circle>` completo en su lugar.
fn armar_rebanadas(rebanadas: &[RebanadaPastel], cx: f64, cy: f64, r: f64) -> Vec<RebanadaVisual> {
    let n = rebanadas.len();
    let mut angulo_acumulado = 0.0;
    rebanadas
        .iter()
        .enumerate()
        .map(|(i, reb)| {
            let fraccion = (decimal_f64(reb.percentage) / 100.0).max(0.0);
            let a0 = angulo_acumulado;
            let a1 = a0 + fraccion * std::f64::consts::TAU;
            angulo_acumulado = a1;
            let centro = (a0 + a1) / 2.0;
            RebanadaVisual {
                camino: if n == 1 {
                    None
                } else {
                    Some(arco_pastel(cx, cy, r, a0, a1))
                },
                centroide: punto_en_circulo(cx, cy, r * 0.66, centro),
                color: color_categoria(i),
                nombre: reb.category_name.clone(),
                monto: reb.amount,
                porcentaje: reb.percentage,
            }
        })
        .collect()
}

const CX: f64 = 110.0;
const CY: f64 = 110.0;
const R: f64 = 90.0;

#[component]
fn Pastel(titulo: &'static str, rebanadas: Vec<RebanadaPastel>) -> impl IntoView {
    if rebanadas.is_empty() {
        return view! {
            <div class="min-w-[220px] flex-1">
                <h3 class="mb-2 text-[13px] font-bold text-text">{titulo}</h3>
                <p class="text-soft text-[13px]">"Sin movimientos."</p>
            </div>
        }
        .into_any();
    }

    let activo = RwSignal::new(None::<usize>);
    let visuales = armar_rebanadas(&rebanadas, CX, CY, R);
    let visuales_detalle = visuales.clone();

    view! {
        <div class="min-w-[220px] flex-1">
            <h3 class="mb-2 text-[13px] font-bold text-text">{titulo}</h3>
            <svg viewBox="0 0 220 220" style="width:100%; max-width:220px; display:block; margin:0 auto;">
                {visuales
                    .iter()
                    .enumerate()
                    .map(|(indice, v)| {
                        let color = v.color;
                        match v.camino.clone() {
                            Some(d) => view! {
                                <path
                                    d=d
                                    style=move || {
                                        let grosor = if activo.get() == Some(indice) { 3 } else { 1 };
                                        format!("fill:{color}; stroke:var(--panel); stroke-width:{grosor}; cursor:pointer;")
                                    }
                                    on:mouseenter=move |_| activo.set(Some(indice))
                                    on:mouseleave=move |_| activo.set(None)
                                    on:click=move |_| {
                                        activo.update(|a| *a = if *a == Some(indice) { None } else { Some(indice) })
                                    }
                                ></path>
                            }
                            .into_any(),
                            None => view! {
                                <circle
                                    cx=f(CX)
                                    cy=f(CY)
                                    r=f(R)
                                    style=format!("fill:{color}; cursor:pointer;")
                                    on:mouseenter=move |_| activo.set(Some(indice))
                                    on:mouseleave=move |_| activo.set(None)
                                    on:click=move |_| {
                                        activo.update(|a| *a = if *a == Some(indice) { None } else { Some(indice) })
                                    }
                                ></circle>
                            }
                            .into_any(),
                        }
                    })
                    .collect_view()}

                {visuales
                    .iter()
                    .filter(|v| decimal_f64(v.porcentaje) >= 6.0)
                    .map(|v| {
                        let (x, y) = v.centroide;
                        view! {
                            <text
                                x=f(x)
                                y=f(y)
                                style="text-anchor:middle; dominant-baseline:middle; font-size:11px; font-weight:700; fill:#fff; pointer-events:none;"
                            >
                                {format!("{:.0}%", v.porcentaje)}
                            </text>
                        }
                    })
                    .collect_view()}
            </svg>

            <p class="mt-2 min-h-[18px] text-center text-[12.5px] text-soft">
                {move || match activo.get() {
                    None => "Toca o pasa el cursor sobre una rebanada".to_string(),
                    Some(i) => {
                        let v = &visuales_detalle[i];
                        format!("{}: {:.2} ({:.1}%)", v.nombre, v.monto, v.porcentaje)
                    }
                }}
            </p>

            <div class="mt-2 flex flex-col gap-1">
                {visuales
                    .iter()
                    .map(|v| {
                        view! {
                            <div class="flex items-center justify-between gap-2 text-[12px] text-muted">
                                <span class="flex items-center gap-1.5 truncate">
                                    <span
                                        class="h-[8px] w-[8px] flex-none rounded-full"
                                        style=format!("background:{};", v.color)
                                    ></span>
                                    <span class="truncate">{v.nombre.clone()}</span>
                                </span>
                                <span class="mono flex-none">{format!("{:.1}%", v.porcentaje)}</span>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
    .into_any()
}

#[component]
pub fn Distribucion(
    workspace_id: Uuid,
    desde: RwSignal<String>,
    hasta: RwSignal<String>,
    alcance: RwSignal<Option<Uuid>>,
) -> impl IntoView {
    let auth = use_auth();

    let datos = LocalResource::new(move || {
        let desde_txt = desde.get();
        let hasta_txt = hasta.get();
        let alcance_actual = alcance.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let desde: Option<NaiveDate> = desde_txt.parse().ok();
            let hasta: Option<NaiveDate> = hasta_txt.parse().ok();
            analytics::flujo_pastel(workspace_id, desde, hasta, alcance_actual, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h2>"Ingresos y gastos por categoría"</h2>
            </div>
            {move || match datos.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(datos)) if datos.ingresos.is_empty() && datos.gastos.is_empty() => {
                    view! { <p class="text-soft">"No hay movimientos en este período."</p> }.into_any()
                }
                Some(Ok(datos)) => view! {
                    <div class="flex flex-wrap gap-6">
                        <Pastel titulo="Ingresos" rebanadas=datos.ingresos/>
                        <Pastel titulo="Gastos" rebanadas=datos.gastos/>
                    </div>
                }
                .into_any(),
            }}
        </section>
    }
}
