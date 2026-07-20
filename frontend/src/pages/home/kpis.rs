//! KPI row: ingresos, egresos y flujo neto del período (backend
//! `analytics::flujo_caja`). El filtro desde/hasta lo controla la
//! página raíz (`home.rs`) y lo comparte con `distribucion.rs`.
//!
//! Los tiles "Ingresos"/"Egresos" son clickeables: alternan
//! `filtro_tipo` (compartido con `home.rs`), que filtra la lista de
//! `AccesosRapidos` por tipo de transacción — sin cruzarse con el
//! rango de fecha de este panel (Accesos rápidos nunca filtró por
//! fecha, ver comentario en ese archivo). "Flujo neto" no es
//! clickeable: no existe un tipo de transacción "neto" que filtrar.

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
    filtro_tipo: RwSignal<Option<String>>,
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
                    let alternar = move |tipo: &'static str| {
                        filtro_tipo.update(|actual| {
                            *actual = if actual.as_deref() == Some(tipo) { None } else { Some(tipo.to_string()) };
                        });
                    };
                    let clase_tile = move |tipo: &'static str| {
                        let activo = filtro_tipo.get().as_deref() == Some(tipo);
                        let hay_filtro = filtro_tipo.get().is_some();
                        match (activo, hay_filtro) {
                            (true, _) => "stat-tile is-clickable",
                            (false, true) => "stat-tile is-clickable is-dimmed",
                            (false, false) => "stat-tile is-clickable",
                        }
                    };
                    let estilo_tile = move |tipo: &'static str, color: &'static str, color_rgb: &'static str| {
                        if filtro_tipo.get().as_deref() == Some(tipo) {
                            format!("border-color:{color}; background:rgba({color_rgb}, .08);")
                        } else {
                            String::new()
                        }
                    };
                    view! {
                        <div class="stat-row">
                            <button
                                type="button"
                                class=move || clase_tile("income")
                                style=move || estilo_tile("income", "var(--positive)", "var(--positive-rgb)")
                                on:click=move |_| alternar("income")
                            >
                                <p class="stat-label">"Ingresos"</p>
                                <p class="stat-value">{format!("{:.2}", f.income)}</p>
                            </button>
                            <button
                                type="button"
                                class=move || clase_tile("expense")
                                style=move || estilo_tile("expense", "var(--negative)", "var(--negative-rgb)")
                                on:click=move |_| alternar("expense")
                            >
                                <p class="stat-label">"Egresos"</p>
                                <p class="stat-value">{format!("{:.2}", f.expense)}</p>
                            </button>
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
