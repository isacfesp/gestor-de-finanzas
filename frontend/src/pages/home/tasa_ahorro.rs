//! Ahorro del período: tasa (%, backend `analytics::tasa_ahorro`, atada
//! al mes en curso) y monto neto (backend `analytics::ahorro_neto`,
//! sobre un rango de fechas libre) — ambos miden aportes a metas (vía
//! `goal_id`, no cuentas `savings` — son conceptos distintos en este
//! proyecto). El meter de % reusa la misma barra `.budget-*` que
//! Presupuestos.

use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::util::primer_dia_mes;
use crate::api::analytics;
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn TasaAhorro(workspace_id: Uuid, alcance: RwSignal<Option<Uuid>>) -> impl IntoView {
    let auth = use_auth();
    let mes = RwSignal::new(primer_dia_mes().to_string()[..7].to_string());
    let desde = RwSignal::new(String::new());
    let hasta = RwSignal::new(String::new());

    let tasa = LocalResource::new(move || {
        let mes_txt = mes.get();
        let alcance_actual = alcance.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let month = format!("{mes_txt}-01").parse().ok();
            analytics::tasa_ahorro(workspace_id, month, alcance_actual, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let neto = LocalResource::new(move || {
        let desde_txt = desde.get();
        let hasta_txt = hasta.get();
        let alcance_actual = alcance.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            analytics::ahorro_neto(
                workspace_id,
                desde_txt.parse().ok(),
                hasta_txt.parse().ok(),
                alcance_actual,
                &token,
            )
            .await
            .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h2>"Tasa de ahorro"</h2>
                <input
                    type="month"
                    prop:value=move || mes.get()
                    on:input=move |ev| mes.set(event_target_value(&ev))
                />
            </div>
            {move || match tasa.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(t)) => {
                    let porcentaje_f64: f64 = t.percentage.to_string().parse().unwrap_or(0.0);
                    let ancho = porcentaje_f64.clamp(0.0, 100.0).round() as u32;
                    view! {
                        <div class="budget-card">
                            <div class="budget-card-head">
                                <h4>"% del ingreso destinado a metas"</h4>
                                <span class="text-soft" style="font-size:12px;">
                                    {format!("{:.2} / {:.2}", t.goal_income, t.total_income)}
                                </span>
                            </div>
                            <div class="budget-track">
                                <div class="budget-fill" style=format!("width:{ancho}%;")></div>
                            </div>
                            <p class="text-faint" style="margin:6px 0 0; font-size:12px;">
                                {format!("{:.0}%", t.percentage)}
                            </p>
                        </div>
                    }
                    .into_any()
                }
            }}

            <div class="panel-head" style="margin-top:16px;">
                <h3 style="font-size:14px;">"Ahorro neto"</h3>
                <div style="display:flex; gap:10px; align-items:center;">
                    <input
                        type="date"
                        prop:value=move || desde.get()
                        on:input=move |ev| desde.set(event_target_value(&ev))
                    />
                    <input
                        type="date"
                        prop:value=move || hasta.get()
                        on:input=move |ev| hasta.set(event_target_value(&ev))
                    />
                </div>
            </div>
            {move || match neto.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(n)) => {
                    let color_neto = if n.neto >= Decimal::ZERO { "var(--positive)" } else { "var(--negative)" };
                    view! {
                        <div class="stat-row">
                            <div class="stat-tile">
                                <p class="stat-label">"Aportado"</p>
                                <p class="stat-value">{format!("{:.2}", n.aportado)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"Retirado"</p>
                                <p class="stat-value">{format!("{:.2}", n.retirado)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"Neto"</p>
                                <p class="stat-value" style=format!("color:{color_neto};")>{format!("{:.2}", n.neto)}</p>
                            </div>
                        </div>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}
