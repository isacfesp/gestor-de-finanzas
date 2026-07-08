//! Página de inicio autenticada. Por ahora solo confirma que el login
//! quedó conectado de punta a punta (muestra los datos que devolvió
//! `GET /auth/yo`); el resumen financiero real llega con el módulo
//! accounting.

use leptos::prelude::*;

use crate::auth::use_auth;

#[component]
pub fn Home() -> impl IntoView {
    let auth = use_auth();

    view! {
        <section class="panel" style="padding: 22px 20px;">
            <p class="eyebrow">"Sesión iniciada"</p>
            {move || {
                auth.usuario()
                    .map(|u| {
                        view! {
                            <p class="figure" style="font-size: 24px; margin: 6px 0 4px;">
                                "Hola, " {u.name}
                            </p>
                            <p class="text-soft">{u.email} " · rol " {u.role}</p>
                        }
                            .into_any()
                    })
                    .unwrap_or_else(|| view! { <p class="text-soft">"Cargando..."</p> }.into_any())
            }}
        </section>

        <section class="panel placeholder-panel">
            <span class="figure">"Resumen"</span>
            <p class="text-soft">"El balance, movimientos recientes y metas aparecerán aquí cuando conectemos el módulo accounting."</p>
        </section>
    }
}
