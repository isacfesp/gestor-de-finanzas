//! Meter de tasa de ahorro del mes (backend `analytics::tasa_ahorro`):
//! % del ingreso destinado a metas (vía `goal_id`, no cuentas
//! `savings` — son conceptos distintos en este proyecto). Reusa la
//! misma barra `.budget-*` que Presupuestos.

use leptos::prelude::*;
use uuid::Uuid;

use super::util::primer_dia_mes;
use crate::api::analytics;
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn TasaAhorro(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let mes = RwSignal::new(primer_dia_mes().to_string()[..7].to_string());

    let tasa = LocalResource::new(move || {
        let mes_txt = mes.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let month = format!("{mes_txt}-01").parse().ok();
            analytics::tasa_ahorro(workspace_id, month, &token)
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
        </section>
    }
}
