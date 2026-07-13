//! Tendencia de ingresos/egresos de los últimos meses — gráfica de
//! línea, pedido explícito de rediseño mobile-first. No existía un
//! equivalente antes: la única métrica de flujo de caja del dashboard
//! era el KPI de un solo total (`Kpis`), sin serie de tiempo. El SVG lo
//! arma `charts-rs` en el backend (`analytics::graficos::tendencia`) y
//! aquí solo se inyecta con `inner_html`.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::analytics;
use crate::auth::{token_vigente, use_auth};
use crate::components::theme::use_theme;

#[component]
pub fn Tendencia(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let tema = use_theme();

    let svg = LocalResource::new(move || {
        let tema_txt = tema.actual().como_texto();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            analytics::tendencia_svg(workspace_id, Some(6), tema_txt, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h2>"Tendencia (últimos 6 meses)"</h2>
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
