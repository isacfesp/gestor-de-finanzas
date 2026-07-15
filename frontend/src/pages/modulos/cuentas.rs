//! Página "Cuentas": cuentas/billeteras (backend `accounts`) y sus
//! transacciones (backend `accounting`) conviviendo en una sola
//! pantalla, con categorías y etiquetas (backend `accounting`/`tags`)
//! en su propia pestaña — ver `docs/frontend-ia.md`.
//!
//! Tres pestañas:
//! - **Cuentas**: tarjetas de cuenta + alta/edición.
//! - **Transacciones**: filtros + tabla + alta/edición + transferencias.
//! - **Categorías y Etiquetas**: gestión de ambas fuera del formulario de
//!   transacción — una categoría/etiqueta se crea aquí, no al vuelo.

use leptos::prelude::*;
use uuid::Uuid;

use crate::workspace::use_workspace;

mod categorias_tab;
mod cuentas_tab;
mod transacciones_tab;
mod util;

use categorias_tab::PestanaCategorias;
use cuentas_tab::PestanaCuentas;
use transacciones_tab::PestanaTransacciones;

// Reexportados para la creación rápida de categoría/cuenta dentro de
// los formularios de operación de otros módulos (Agenda) sin salir de
// pantalla — ver `agenda::previstos_tab`/`agenda::suscripciones_tab`.
pub(crate) use categorias_tab::FormularioCategoria;
pub(crate) use cuentas_tab::FormularioCuenta;

#[derive(Copy, Clone, PartialEq, Eq)]
enum Pestana {
    Cuentas,
    Transacciones,
    CategoriasEtiquetas,
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
            <Show when=move || pestana.get() == Pestana::CategoriasEtiquetas>
                <PestanaCategorias workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
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
            <button
                class=move || clase(Pestana::CategoriasEtiquetas)
                on:click=move |_| pestana.set(Pestana::CategoriasEtiquetas)
            >
                "Categorías y Etiquetas"
            </button>
        </div>
    }
}
