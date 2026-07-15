//! KPI row: ingresos, egresos y flujo neto del período (backend
//! `analytics::flujo_caja`). El filtro desde/hasta lo controla la
//! página raíz (`home.rs`) y lo comparte con `distribucion.rs`.

use chrono::NaiveDate;
use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::api::analytics;
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn Kpis(
    workspace_id: Uuid,
    desde: RwSignal<String>,
    hasta: RwSignal<String>,
    alcance: RwSignal<Option<Uuid>>,
) -> impl IntoView {
    let auth = use_auth();

    let flujo = LocalResource::new(move || {
        let desde_txt = desde.get();
        let hasta_txt = hasta.get();
        let alcance_actual = alcance.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let desde: Option<NaiveDate> = desde_txt.parse().ok();
            let hasta: Option<NaiveDate> = hasta_txt.parse().ok();
            analytics::flujo_caja(workspace_id, desde, hasta, alcance_actual, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Flujo de caja"</h2>
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
            {move || match flujo.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(f)) => {
                    let color_neto = if f.net >= Decimal::ZERO { "var(--positive)" } else { "var(--negative)" };
                    view! {
                        <div class="stat-row">
                            <div class="stat-tile">
                                <p class="stat-label">"Ingresos"</p>
                                <p class="stat-value">{format!("{:.2}", f.income)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"Egresos"</p>
                                <p class="stat-value">{format!("{:.2}", f.expense)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"Flujo neto"</p>
                                <p class="stat-value" style=format!("color:{color_neto};")>{format!("{:.2}", f.net)}</p>
                            </div>
                        </div>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}
