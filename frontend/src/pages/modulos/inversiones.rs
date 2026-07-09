//! Página "Inversiones": metas de ahorro (backend `goals`) e
//! inversiones SOFIPO (backend `investments`) — "dinero apartado para
//! el futuro", ver `docs/frontend-ia.md`.
//!
//! Tres pestañas:
//! - **Metas**: listado con progreso, alta/edición, aportes y
//!   proyección de ahorro.
//! - **Inversiones**: activas/vencidas, alta, y detalle con
//!   rendimiento bruto/ISR/neto e historial de rendimientos reales.
//! - **Simulador**: calculadora libre de rendimiento, no persiste nada.

use leptos::prelude::*;
use uuid::Uuid;

use crate::workspace::use_workspace;

mod desglose;
mod inversiones_tab;
mod metas_tab;
mod simulador_tab;
mod util;

use inversiones_tab::PestanaInversiones;
use metas_tab::PestanaMetas;
use simulador_tab::PestanaSimulador;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Pestana {
    Metas,
    Inversiones,
    Simulador,
}

#[component]
pub fn InversionesPage() -> impl IntoView {
    let workspace = use_workspace();
    let pestana = RwSignal::new(Pestana::Metas);

    view! {
        <Show
            when=move || workspace.id().is_some()
            fallback=move || {
                view! {
                    <section class="panel">
                        <p class="text-soft">
                            {move || workspace.error().unwrap_or_else(|| "Cargando workspace...".to_string())}
                        </p>
                    </section>
                }
            }
        >
            <BarraPestanas pestana=pestana/>
            // workspace.id() nunca es None aquí (el <Show> de arriba ya lo
            // garantiza); Uuid::nil() es solo un valor de respaldo inerte
            // para no depender de .unwrap()/.expect().
            <Show when=move || pestana.get() == Pestana::Metas>
                <PestanaMetas workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Inversiones>
                <PestanaInversiones workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Simulador>
                <PestanaSimulador workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
        </Show>
    }
}

#[component]
fn BarraPestanas(pestana: RwSignal<Pestana>) -> impl IntoView {
    let clase = move |p: Pestana| {
        if pestana.get() == p {
            "tab-btn is-active"
        } else {
            "tab-btn"
        }
    };

    view! {
        <div class="tabs">
            <button class=move || clase(Pestana::Metas) on:click=move |_| pestana.set(Pestana::Metas)>
                "Metas"
            </button>
            <button class=move || clase(Pestana::Inversiones) on:click=move |_| pestana.set(Pestana::Inversiones)>
                "Inversiones"
            </button>
            <button class=move || clase(Pestana::Simulador) on:click=move |_| pestana.set(Pestana::Simulador)>
                "Simulador"
            </button>
        </div>
    }
}
