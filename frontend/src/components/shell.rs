//! Armazón visual de las páginas autenticadas: encabezado, navegación
//! (barra inferior en móvil, rail lateral en escritorio a partir del
//! breakpoint `md:` de Tailwind) y el `<Outlet/>` donde el router
//! dibuja cada página. También hace de guardia de autenticación: si no
//! hay sesión, redirige a `/login` en vez de mostrar el contenido.

use leptos::prelude::*;
use leptos_router::components::{A, Outlet, Redirect};
use uuid::Uuid;

use crate::auth::{cerrar_sesion, use_auth};
use crate::components::boton_rapido::BotonRapido;
use crate::components::notificaciones::CampanaNotificaciones;
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

/// Compartida por los links de `NAV_ITEMS` y el link de Admin: móvil
/// (base) apila ícono+etiqueta en columna dentro de la barra inferior;
/// `md:` los pone en fila dentro del rail lateral. `aria-[current=page]`
/// es el estado activo que ya pone `<A exact=true>` de leptos_router.
const NAV_LINK_CLASS: &str = "flex flex-none flex-col items-center justify-center gap-[3px] \
    rounded-sm px-3.5 py-1.5 min-w-[64px] text-[10.5px] font-semibold text-muted \
    transition-colors hover:bg-hover aria-[current=page]:bg-accent-soft \
    aria-[current=page]:text-accent md:min-w-0 md:flex-row md:justify-start md:gap-3 \
    md:px-3 md:py-[11px] md:text-sm";

const NAV_ICON_CLASS: &str = "w-[19px] h-[19px] flex-none fill-none stroke-current [stroke-linecap:round] [stroke-linejoin:round] [stroke-width:1.8]";

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
        "shield" => view! {
            <path d="M12 3.5 19 6.5v5c0 5-3.2 7.8-7 9-3.8-1.2-7-4-7-9v-5z"></path><path d="M9 12l2 2 4-4.5"></path>
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
    // entre ellas). Ver `crate::workspace::cargar_activo`.
    Effect::new(move |_| {
        if auth.is_logged_in() {
            leptos::task::spawn_local(async move {
                crate::workspace::cargar_activo(auth, workspace).await;
            });
        }
    });

    view! {
        <Show when=move || auth.is_logged_in() fallback=|| view! { <Redirect path="/login"/> }>
            <div class="flex min-h-[100dvh] flex-col bg-bg md:flex-row">
                <nav class="fixed inset-x-0 bottom-0 z-[5] flex flex-row items-center gap-1 overflow-x-auto overflow-y-visible border-t border-line bg-sidebar px-2.5 pt-2 pb-[calc(8px+env(safe-area-inset-bottom))] md:sticky md:inset-auto md:top-0 md:h-[100dvh] md:w-[250px] md:flex-none md:flex-col md:items-stretch md:gap-0.5 md:overflow-x-visible md:overflow-y-auto md:border-t-0 md:border-r md:px-4 md:py-[22px]">
                    <div class="hidden items-center gap-2.5 px-2 pb-[22px] pt-1.5 md:flex">
                        <span class="brand-mark">"G"</span>
                        <span class="text-[17px] font-extrabold tracking-[-0.02em] text-text">"Geck-gestor"</span>
                    </div>

                    <span class="hidden px-2.5 pb-1.5 pt-[18px] font-mono text-[10px] font-semibold tracking-[0.08em] text-faint md:block">"GENERAL"</span>
                    {NAV_ITEMS
                        .iter()
                        .map(|(label, path, kind)| {
                            view! {
                                <A href=*path exact=true attr:class=NAV_LINK_CLASS>
                                    <svg viewBox="0 0 24 24" class=NAV_ICON_CLASS>{icono(kind)}</svg>
                                    <span>{*label}</span>
                                </A>
                            }
                        })
                        .collect_view()}

                    // Admin no vive en NAV_ITEMS (esa lista es estática,
                    // sin lógica) porque solo el rol dev debe verlo.
                    <Show when=move || auth.es_dev()>
                        <A href="/admin" exact=true attr:class=NAV_LINK_CLASS>
                            <svg viewBox="0 0 24 24" class=NAV_ICON_CLASS>{icono("shield")}</svg>
                            <span>"Admin"</span>
                        </A>
                    </Show>

                    <div class="hidden md:mt-auto md:block md:overflow-hidden md:rounded-[14px] md:border md:border-line md:p-4 md:[background:linear-gradient(150deg,#123a63,#0a1a3a)]">
                        <p class="mb-1 text-[13px] font-bold text-[#eaf0ff]">"¿Necesitas ayuda?"</p>
                        <p class="mb-3 text-[12px] text-[rgba(234,240,255,.6)]">"Revisa la guía de geck"</p>
                        <button class="w-full rounded-sm bg-[rgba(255,255,255,.08)] px-[13px] py-[9px] text-[12.5px] font-semibold text-[#eaf0ff] hover:bg-[rgba(255,255,255,.14)]">
                            "Documentación"
                        </button>
                    </div>
                </nav>

                <div class="flex min-w-0 flex-1 flex-col">
                    <header class="sticky top-0 z-20 flex items-center gap-4 border-b border-line px-4 pb-4 pt-[calc(16px+env(safe-area-inset-top))] md:px-[30px] md:pb-[18px] md:pt-[calc(18px+env(safe-area-inset-top))]">
                        <Show when=move || workspace.nombre().is_some()>
                            <span class="font-mono text-[11px] font-semibold text-faint">{move || workspace.nombre().unwrap_or_default()}</span>
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
                        <Show when=move || workspace.id().is_some()>
                            <CampanaNotificaciones workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
                        </Show>
                        <A href="/perfil" attr:class="flex items-center gap-[9px] rounded-sm border border-card-line bg-transparent py-[5px] pr-[10px] pl-[6px] text-text hover:bg-hover">
                            <span class="flex h-[30px] w-[30px] items-center justify-center rounded-[8px] bg-[linear-gradient(135deg,#8b5cf6,var(--accent))] text-[13px] font-bold text-[#04222e]">
                                {move || auth.usuario().map(|u| iniciales(&u.name)).unwrap_or_default()}
                            </span>
                            <span class="hidden sm:inline">{move || auth.usuario().map(|u| u.name).unwrap_or_default()}</span>
                        </A>
                        <button class="app-icon-btn" title="Cerrar sesión" on:click=salir>
                            <svg viewBox="0 0 24 24">
                                <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"></path>
                                <path d="M16 17l5-5-5-5"></path>
                                <path d="M21 12H9"></path>
                            </svg>
                        </button>
                    </header>

                    <main class="min-w-0 flex-1 px-4 pt-5 pb-[90px] md:px-[30px] md:pt-[26px] md:pb-10">
                        <Outlet/>
                    </main>
                </div>

                <BotonRapido/>
            </div>
        </Show>
    }
}
