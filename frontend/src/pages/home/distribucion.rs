//! Distribución de gastos por categoría, como pastel — pedido
//! explícito de rediseño mobile-first que **reemplaza** la barra
//! horizontal apilada que llevaba esta sección (la convención "nunca
//! donut" de la skill `dataviz` seguía documentada para esa barra; el
//! pastel es una decisión de diseño nueva, no un descuido). El SVG lo
//! arma `charts-rs` en el backend (`analytics::graficos::flujo_pastel`,
//! con la misma agregación por categoría que antes hacía esta vista) y
//! aquí solo se inyecta con `inner_html`. Comparte el filtro
//! desde/hasta con `Kpis`.

use chrono::NaiveDate;
use leptos::prelude::*;
use uuid::Uuid;

use crate::api::analytics;
use crate::auth::{token_vigente, use_auth};
use crate::components::theme::use_theme;

#[component]
pub fn Distribucion(
    workspace_id: Uuid,
    desde: RwSignal<String>,
    hasta: RwSignal<String>,
    alcance: RwSignal<Option<Uuid>>,
) -> impl IntoView {
    let auth = use_auth();
    let tema = use_theme();

    let svg = LocalResource::new(move || {
        let desde_txt = desde.get();
        let hasta_txt = hasta.get();
        let tema_txt = tema.actual().como_texto();
        let alcance_actual = alcance.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let desde: Option<NaiveDate> = desde_txt.parse().ok();
            let hasta: Option<NaiveDate> = hasta_txt.parse().ok();
            analytics::flujo_pastel_svg(
                workspace_id,
                desde,
                hasta,
                tema_txt,
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
                <h2>"Ingresos y gastos por categoría"</h2>
            </div>
            {move || match svg.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(marcado)) if marcado.is_empty() => {
                    view! { <p class="text-soft">"No hay movimientos en este período."</p> }.into_any()
                }
                Some(Ok(marcado)) => {
                    view! { <div class="flex justify-center" inner_html=marcado></div> }.into_any()
                }
            }}
        </section>
    }
}
