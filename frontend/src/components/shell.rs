//! Armazón visual de las páginas autenticadas: encabezado, navegación
//! (barra inferior en móvil, rail lateral en escritorio vía media
//! query en `main.css`) y el `<Outlet/>` donde el router dibuja cada
//! página. También hace de guardia de autenticación: si no hay
//! sesión, redirige a `/login` en vez de mostrar el contenido.

use leptos::prelude::*;
use leptos_router::components::{A, Outlet, Redirect};

use crate::auth::{cerrar_sesion, use_auth};
use crate::components::theme::use_theme;

const NAV_ITEMS: &[(&str, &str, &str)] = &[
    ("Inicio", "/", "home"),
    ("Movimientos", "/movimientos", "list"),
    ("Cuentas", "/cuentas", "wallet"),
    ("Metas", "/metas", "target"),
    ("Inversiones", "/inversiones", "trend"),
    ("Etiquetas", "/etiquetas", "tag"),
    ("Previstos", "/previstos", "calendar"),
];

fn icono(kind: &str) -> AnyView {
    match kind {
        "home" => view! {
            <path d="M4 11.5 12 4l8 7.5"></path><path d="M6 10v9h5v-6h2v6h5v-9"></path>
        }
        .into_any(),
        "list" => view! {
            <rect x="5" y="4" width="14" height="16" rx="2"></rect><path d="M8.5 9h7M8.5 13h7M8.5 17h4"></path>
        }
        .into_any(),
        "wallet" => view! {
            <rect x="3.5" y="6" width="17" height="12" rx="2"></rect><path d="M3.5 10h17"></path><circle cx="16.5" cy="14" r="1"></circle>
        }
        .into_any(),
        "target" => view! {
            <circle cx="12" cy="12" r="7.5"></circle><circle cx="12" cy="12" r="3.4"></circle>
        }
        .into_any(),
        "trend" => view! {
            <path d="M4 16l5-5 4 4 7-8"></path><path d="M15 7h5v5"></path>
        }
        .into_any(),
        "tag" => view! {
            <path d="M12 3h6a2 2 0 0 1 2 2v6l-9 9-8-8z"></path><circle cx="15.5" cy="7.5" r="1.2"></circle>
        }
        .into_any(),
        _ => view! {
            <rect x="4" y="5" width="16" height="15" rx="2"></rect><path d="M4 9h16M8 3v4M16 3v4"></path>
        }
        .into_any(),
    }
}

/// Primeras dos iniciales del nombre, para el avatar del encabezado.
fn iniciales(nombre: &str) -> String {
    nombre
        .split_whitespace()
        .filter_map(|palabra| palabra.chars().next())
        .take(2)
        .collect::<String>()
        .to_uppercase()
}

#[component]
pub fn ProtectedShell() -> impl IntoView {
    let auth = use_auth();
    let tema = use_theme();

    let salir = move |_| {
        leptos::task::spawn_local(async move {
            cerrar_sesion(auth).await;
        });
    };

    view! {
        <Show when=move || auth.is_logged_in() fallback=|| view! { <Redirect path="/login"/> }>
            <div class="app-shell">
                <nav class="app-nav">
                    {NAV_ITEMS
                        .iter()
                        .map(|(label, path, kind)| {
                            view! {
                                <A href=*path exact=true attr:class="nav-link">
                                    <svg viewBox="0 0 24 24">{icono(kind)}</svg>
                                    <span>{*label}</span>
                                </A>
                            }
                        })
                        .collect_view()}
                </nav>

                <header class="app-header">
                    <span class="brand-mark"></span>
                    <button
                        class="btn-ghost"
                        style="border:none; padding:4px 8px;"
                        on:click=move |_| tema.alternar()
                    >
                        {move || match tema.actual() {
                            crate::components::theme::Tema::Oscuro => "Oscuro",
                            crate::components::theme::Tema::Claro => "Claro",
                        }}
                    </button>
                    <button class="app-user-btn" on:click=salir>
                        <span class="avatar">
                            {move || auth.usuario().map(|u| iniciales(&u.name)).unwrap_or_default()}
                        </span>
                        <span>"Salir"</span>
                    </button>
                </header>

                <main class="app-main">
                    <Outlet/>
                </main>
            </div>
        </Show>
    }
}
