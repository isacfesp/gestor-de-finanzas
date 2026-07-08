//! Armazón visual de las páginas autenticadas: encabezado, navegación
//! (barra inferior en móvil, rail lateral en escritorio vía media
//! query en `main.css`) y el `<Outlet/>` donde el router dibuja cada
//! página. También hace de guardia de autenticación: si no hay
//! sesión, redirige a `/login` en vez de mostrar el contenido.

use leptos::prelude::*;
use leptos_router::components::{A, Outlet, Redirect};

use crate::auth::{cerrar_sesion, use_auth};
use crate::components::theme::use_theme;
use crate::workspace::use_workspace;

/// Las 5 secciones de navegación de `docs/frontend-ia.md` — no es 1:1
/// con los módulos del backend, ver ese documento.
const NAV_ITEMS: &[(&str, &str, &str)] = &[
    ("Inicio", "/", "home"),
    ("Cuentas", "/cuentas", "wallet"),
    ("Agenda", "/agenda", "calendar"),
    ("Inversiones", "/inversiones", "trend"),
    ("Movimientos", "/movimientos", "list"),
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
        "trend" => view! {
            <path d="M4 16l5-5 4 4 7-8"></path><path d="M15 7h5v5"></path>
        }
        .into_any(),
        // "calendar" (Agenda) y cualquier otro caso caen aquí.
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
    let workspace = use_workspace();

    let salir = move |_| {
        leptos::task::spawn_local(async move {
            cerrar_sesion(auth).await;
        });
    };

    // Se resuelve una sola vez, en cuanto hay sesión (el shell envuelve
    // todas las rutas protegidas y no se vuelve a montar al navegar
    // entre ellas). Ver `crate::workspace` — es un atajo interino
    // mientras no exista un endpoint de autoservicio.
    Effect::new(move |_| {
        if auth.is_logged_in() {
            leptos::task::spawn_local(async move {
                crate::workspace::cargar_activo(auth, workspace).await;
            });
        }
    });

    view! {
        <Show when=move || auth.is_logged_in() fallback=|| view! { <Redirect path="/login"/> }>
            <div class="app-shell">
                <nav class="app-nav">
                    <div class="app-nav-brand">
                        <span class="brand-mark">"g"</span>
                        <span class="brand-word">"geck"</span>
                    </div>

                    <span class="app-nav-section">"GENERAL"</span>
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

                    <div class="app-nav-help">
                        <p>"¿Necesitas ayuda?"</p>
                        <p class="text-soft">"Revisa la guía de geck"</p>
                        <button class="btn-ghost">"Documentación"</button>
                    </div>
                </nav>

                <div style="flex:1; min-width:0; display:flex; flex-direction:column;">
                    <header class="app-header">
                        <Show when=move || workspace.nombre().is_some()>
                            <span class="app-workspace">{move || workspace.nombre().unwrap_or_default()}</span>
                        </Show>
                        <button class="app-icon-btn" title="Cambiar tema" on:click=move |_| tema.alternar()>
                            {move || match tema.actual() {
                                crate::components::theme::Tema::Oscuro => view! {
                                    <svg viewBox="0 0 24 24"><path d="M21 12.8A8.5 8.5 0 1 1 11.2 3a6.6 6.6 0 0 0 9.8 9.8z"></path></svg>
                                }.into_any(),
                                crate::components::theme::Tema::Claro => view! {
                                    <svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="4.5"></circle><path d="M12 3v2M12 19v2M4.2 4.2l1.4 1.4M18.4 18.4l1.4 1.4M3 12h2M19 12h2M4.2 19.8l1.4-1.4M18.4 5.6l1.4-1.4"></path></svg>
                                }.into_any(),
                            }}
                        </button>
                        <button class="app-user-btn">
                            <span class="avatar">
                                {move || auth.usuario().map(|u| iniciales(&u.name)).unwrap_or_default()}
                            </span>
                            <span>{move || auth.usuario().map(|u| u.name).unwrap_or_default()}</span>
                        </button>
                        <button class="app-icon-btn" title="Cerrar sesión" on:click=salir>
                            <svg viewBox="0 0 24 24">
                                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
                                <path d="M16 17l5-5-5-5"></path>
                                <path d="M21 12H9"></path>
                            </svg>
                        </button>
                    </header>

                    <main class="app-main">
                        <Outlet/>
                    </main>
                </div>
            </div>
        </Show>
    }
}
