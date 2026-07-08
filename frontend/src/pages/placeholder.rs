//! Panel reutilizable para los módulos que todavía no tienen pantalla
//! propia. Cada página de `pages::modulos` es, por ahora, un envoltorio
//! delgado alrededor de este componente con su título.

use leptos::prelude::*;

#[component]
pub fn Placeholder(titulo: &'static str, descripcion: &'static str) -> impl IntoView {
    view! {
        <section class="panel placeholder-panel">
            <span class="figure">{titulo}</span>
            <p class="text-soft">{descripcion}</p>
        </section>
    }
}
