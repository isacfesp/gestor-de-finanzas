//! Página "Cuentas": cuentas/billeteras (backend `accounts`) y sus
//! transacciones (backend `accounting`) conviviendo en una sola
//! pantalla, con etiquetas (backend `tags`) como apoyo al registrar una
//! transacción — ver `docs/frontend-ia.md`.
//!
//! Dos pestañas:
//! - **Cuentas**: tarjetas de cuenta + alta/edición + transferencias.
//! - **Transacciones**: filtros + tabla + alta/edición.

use leptos::prelude::*;
use uuid::Uuid;

use crate::workspace::use_workspace;

mod cuentas_tab;
mod transacciones_tab;
mod util;

use cuentas_tab::PestanaCuentas;
use transacciones_tab::PestanaTransacciones;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Pestana {
    Cuentas,
    Transacciones,
}

#[component]
pub fn CuentasPage() -> impl IntoView {
    let workspace = use_workspace();
    let pestana = RwSignal::new(Pestana::Cuentas);

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
            <Show when=move || pestana.get() == Pestana::Cuentas>
                <PestanaCuentas workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            </Show>
            <Show when=move || pestana.get() == Pestana::Transacciones>
                <PestanaTransacciones workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
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
            <button class=move || clase(Pestana::Cuentas) on:click=move |_| pestana.set(Pestana::Cuentas)>
                "Cuentas"
            </button>
            <button class=move || clase(Pestana::Transacciones) on:click=move |_| pestana.set(Pestana::Transacciones)>
                "Transacciones"
            </button>
        </div>
    }
}
