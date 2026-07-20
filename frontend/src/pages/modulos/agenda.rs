//! Página "Agenda": suscripciones, presupuestos, previstos (backend
//! `accounting` + `planned_transactions`) y un calendario que junta
//! esos dos con las metas de Inversiones — ver `docs/frontend-ia.md`.
//!
//! Cuatro pestañas: Calendario, Suscripciones, Presupuestos, Previstos.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use uuid::Uuid;

use crate::workspace::use_workspace;

mod calendario_tab;
mod presupuestos_tab;
mod previstos_tab;
mod suscripciones_tab;
mod util;

use calendario_tab::PestanaCalendario;
use presupuestos_tab::PestanaPresupuestos;
use previstos_tab::PestanaPrevistos;
use suscripciones_tab::PestanaSuscripciones;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Pestana {
    Calendario,
    Suscripciones,
    Presupuestos,
    Previstos,
}

#[component]
pub fn AgendaPage() -> impl IntoView {
    let workspace = use_workspace();
    // Deep-link del FAB "Acceso rápido" (`?tab=previstos&crear=1`) — ver
    // el mismo patrón en `modulos::cuentas::CuentasPage`.
    let query = use_query_map();
    let pestana = RwSignal::new(
        if query.with_untracked(|q| q.get("tab")).as_deref() == Some("previstos") {
            Pestana::Previstos
        } else {
            Pestana::Calendario
        },
    );
    let abrir_formulario_inicial = query.with_untracked(|q| q.get("crear")).as_deref() == Some("1");

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
            <Show when=move || pestana.get() == Pestana::Calendario>
                <PestanaCalendario workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Suscripciones>
                <PestanaSuscripciones workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Presupuestos>
                <PestanaPresupuestos workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Previstos>
                <PestanaPrevistos
                    workspace_id=workspace.id().unwrap_or(Uuid::nil())
                    abrir_formulario_inicial=abrir_formulario_inicial
                />
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
            <button class=move || clase(Pestana::Calendario) on:click=move |_| pestana.set(Pestana::Calendario)>
                "Calendario"
            </button>
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
