//! Página "Agenda": suscripciones, presupuestos y previstos (backend
//! `accounting` + `planned_transactions`) — ver `docs/frontend-ia.md`.
//!
//! Tres pestañas: Suscripciones, Presupuestos, Previstos.

use leptos::prelude::*;
use uuid::Uuid;

use crate::workspace::use_workspace;

mod presupuestos_tab;
mod previstos_tab;
mod suscripciones_tab;
mod util;

use presupuestos_tab::PestanaPresupuestos;
use previstos_tab::PestanaPrevistos;
use suscripciones_tab::PestanaSuscripciones;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Pestana {
    Suscripciones,
    Presupuestos,
    Previstos,
}

#[component]
pub fn AgendaPage() -> impl IntoView {
    let workspace = use_workspace();
    let pestana = RwSignal::new(Pestana::Suscripciones);

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
            <Show when=move || pestana.get() == Pestana::Suscripciones>
                <PestanaSuscripciones workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Presupuestos>
                <PestanaPresupuestos workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Previstos>
                <PestanaPrevistos workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
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
            <button class=move || clase(Pestana::Suscripciones) on:click=move |_| pestana.set(Pestana::Suscripciones)>
                "Suscripciones"
            </button>
            <button class=move || clase(Pestana::Presupuestos) on:click=move |_| pestana.set(Pestana::Presupuestos)>
                "Presupuestos"
            </button>
            <button class=move || clase(Pestana::Previstos) on:click=move |_| pestana.set(Pestana::Previstos)>
                "Previstos"
            </button>
        </div>
    }
}
