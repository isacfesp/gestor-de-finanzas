//! Pestaña "Simulador": calculadora libre de rendimiento (backend
//! `investments::inversiones::simular`), sin persistir nada — no hay
//! botón "guardar como inversión real", el checklist es explícito en
//! que este formulario es "libre, sin guardar".

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use uuid::Uuid;

use super::desglose::TarjetaDesglose;
use crate::api::investments::{self, DatosSimulacion, DesgloseRendimiento};
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn PestanaSimulador(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let principal = RwSignal::new(String::new());
    let tasa = RwSignal::new(String::new());
    let tipo_interes = RwSignal::new("simple".to_string());
    let plazo = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let calculando = RwSignal::new(false);
    let resultado = RwSignal::new(None::<DesgloseRendimiento>);

    let calcular = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);
        resultado.set(None);

        let Ok(principal_val) = principal.get_untracked().parse() else {
            error.set(Some("El capital no es un número válido".to_string()));
            return;
        };
        let Ok(tasa_val) = tasa.get_untracked().parse() else {
            error.set(Some("La tasa no es un número válido".to_string()));
            return;
        };
        let Ok(plazo_val) = plazo.get_untracked().parse() else {
            error.set(Some("El plazo no es un número válido".to_string()));
            return;
        };

        calculando.set(true);
        let tipo = tipo_interes.get_untracked();
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                calculando.set(false);
                return;
            };
            let datos = DatosSimulacion {
                principal: principal_val,
                gat_annual_rate: tasa_val,
                interest_type: tipo,
                term_days: plazo_val,
            };
            let resultado_api = investments::simular_inversion(workspace_id, &datos, &token).await;
            calculando.set(false);
            match resultado_api {
                Ok(desglose) => resultado.set(Some(desglose)),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Simulador de rendimiento"</h2>
            </div>
            <p class="text-soft" style="margin-top:0;">
                "Calcula el rendimiento bruto, ISR y neto sin registrar ninguna inversión."
            </p>

            <form class="panel form-panel" on:submit=calcular>
                <div class="form-grid">
                    <div class="field">
                        <label>"Capital"</label>
                        <input
                            placeholder="0.00"
                            prop:value=move || principal.get()
                            on:input=move |ev| principal.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field">
                        <label>"Tasa GAT anual (%)"</label>
                        <input
                            placeholder="0.00"
                            prop:value=move || tasa.get()
                            on:input=move |ev| tasa.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field">
                        <label>"Tipo de interés"</label>
                        <select
                            prop:value=move || tipo_interes.get()
                            on:change=move |ev| tipo_interes.set(event_target_value(&ev))
                        >
                            {investments::TIPOS_INTERES
                                .iter()
                                .map(|(valor, etiqueta)| view! { <option value=*valor>{*etiqueta}</option> })
                                .collect_view()}
                        </select>
                    </div>
                    <div class="field">
                        <label>"Plazo (días)"</label>
                        <input
                            placeholder="0"
                            prop:value=move || plazo.get()
                            on:input=move |ev| plazo.set(event_target_value(&ev))
                        />
                    </div>
                </div>

                <Show when=move || error.get().is_some()>
                    <p class="banner banner-error" style="margin-bottom:14px;">
                        {move || error.get().unwrap_or_default()}
                    </p>
                </Show>

                <div class="form-actions">
                    <button type="submit" class="btn btn-primary" disabled=move || calculando.get()>
                        {move || if calculando.get() { "Calculando..." } else { "Calcular" }}
                    </button>
                </div>
            </form>

            <Show when=move || resultado.get().is_some()>
                <div style="margin-top:16px;">
                    {move || resultado.get().map(|d| view! { <TarjetaDesglose desglose=d/> })}
                </div>
            </Show>
        </section>
    }
}
