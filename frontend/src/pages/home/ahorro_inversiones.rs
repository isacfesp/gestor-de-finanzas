//! Ahorro por inversiones: rendimiento bruto/ISR/neto acumulado de las
//! inversiones activas (backend `investments::resumen`). Distinto de
//! `TasaAhorro`, que mide aportes a metas — este widget es sobre
//! rendimiento de inversiones, un concepto separado en este proyecto.

use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::api::investments;
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn AhorroInversiones(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let resumen = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        investments::resumen_ahorro_inversiones(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    view! {
        <section class="panel" style="margin-top:16px;">
            <div class="panel-head">
                <h2>"Ahorro por inversiones"</h2>
            </div>
            <p class="text-soft" style="margin:0 0 12px; font-size:12.5px;">
                "Rendimiento acumulado de tus inversiones activas"
            </p>
            {move || match resumen.get() {
                None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(r)) => {
                    let color_neto = if r.net_yield_acumulado >= Decimal::ZERO { "var(--positive)" } else { "var(--negative)" };
                    view! {
                        <div class="stat-row">
                            <div class="stat-tile">
                                <p class="stat-label">"Capital invertido"</p>
                                <p class="stat-value">{format!("{:.2}", r.principal_invertido)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"Rendimiento bruto"</p>
                                <p class="stat-value">{format!("{:.2}", r.gross_yield_acumulado)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"ISR retenido"</p>
                                <p class="stat-value" style="color:var(--negative);">{format!("{:.2}", r.isr_acumulado)}</p>
                            </div>
                            <div class="stat-tile">
                                <p class="stat-label">"Rendimiento neto"</p>
                                <p class="stat-value" style=format!("color:{color_neto};")>{format!("{:.2}", r.net_yield_acumulado)}</p>
                            </div>
                        </div>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}
