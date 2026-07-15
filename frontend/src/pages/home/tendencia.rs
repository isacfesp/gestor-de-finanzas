//! Tendencia de ingresos/egresos — gráfica de línea, con 3 vistas
//! elegibles por el usuario: "Semanal" (la semana en curso, día por
//! día), "Mensual" (el mes en curso, semana por semana) y "Anual"
//! (los últimos 12 meses, mes por mes). El rango no es configurable
//! por separado de la vista — cada una fija ambos (ver
//! `analytics::graficos::rango_y_grilla` en el backend). El SVG lo
//! arma `charts-rs` en el backend (`analytics::graficos::tendencia`) y
//! aquí solo se inyecta con `inner_html`.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::analytics;
use crate::auth::{token_vigente, use_auth};
use crate::components::theme::use_theme;

fn titulo_vista(granularidad: &str) -> &'static str {
    match granularidad {
        "semana" => "Tendencia (esta semana)",
        "año" => "Tendencia (últimos 12 meses)",
        _ => "Tendencia (este mes)",
    }
}

#[component]
pub fn Tendencia(workspace_id: Uuid, alcance: RwSignal<Option<Uuid>>) -> impl IntoView {
    let auth = use_auth();
    let tema = use_theme();
    let granularidad = RwSignal::new("mes".to_string());

    let svg = LocalResource::new(move || {
        let tema_txt = tema.actual().como_texto();
        let alcance_actual = alcance.get();
        let granularidad_actual = granularidad.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            analytics::tendencia_svg(
                workspace_id,
                tema_txt,
                alcance_actual,
                &granularidad_actual,
                &token,
            )
            .await
            .map_err(|e| e.to_string())
        }
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
            {move || match svg.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(marcado)) if marcado.is_empty() => {
                    view! { <p class="text-soft">"Sin movimientos en este período."</p> }.into_any()
                }
                Some(Ok(marcado)) => {
                    view! { <div class="overflow-x-auto" inner_html=marcado></div> }.into_any()
                }
            }}
        </section>
    }
}
