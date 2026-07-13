//! Campana de notificaciones del topbar (backend `reminders`): badge
//! de no leídas + dropdown con la lista, reusando el mismo patrón
//! `Portal` + `abrir_menu`/`estilo_posicion` que los menús de fila
//! (ver `menu_flotante.rs` — es genérico, no depende de que el botón
//! esté dentro de una tabla).

use leptos::portal::Portal;
use leptos::prelude::*;
use uuid::Uuid;

use crate::api::reminders;
use crate::auth::{token_vigente, use_auth};
use crate::components::menu_flotante::{PosicionMenu, abrir_menu, estilo_posicion};

#[component]
pub fn CampanaNotificaciones(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let abierto = RwSignal::new(false);
    let posicion: PosicionMenu = RwSignal::new((0.0_f64, 0.0_f64));

    let no_leidas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        reminders::listar_notificaciones(workspace_id, Some(false), &token)
            .await
            .unwrap_or_default()
    });

    let marcar = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = reminders::marcar_leida(workspace_id, id, &token).await;
                no_leidas.refetch();
            }
        });
    };

    view! {
        <button
            type="button"
            class="app-icon-btn app-notif-btn"
            title="Notificaciones"
            on:click=move |ev| abrir_menu(ev, abierto, posicion, 340.0)
        >
            <svg viewBox="0 0 24 24">
                <path d="M6 8a6 6 0 0 1 12 0c0 4 1.5 6 2 7H4c.5-1 2-3 2-7Z"></path>
                <path d="M10 20a2 2 0 0 0 4 0"></path>
            </svg>
            {move || {
                let cantidad = no_leidas.get().map(|l| l.len()).unwrap_or(0);
                if cantidad > 0 {
                    view! { <span class="notif-badge">{cantidad}</span> }.into_any()
                } else {
                    ().into_any()
                }
            }}
        </button>
        <Show when=move || abierto.get()>
            <Portal>
                <div class="notif-dropdown" style=move || estilo_posicion(posicion) on:mouseleave=move |_| abierto.set(false)>
                    {move || match no_leidas.get() {
                        None => view! { <p class="notif-empty">"Cargando..."</p> }.into_any(),
                        Some(lista) if lista.is_empty() => {
                            view! { <p class="notif-empty">"Sin notificaciones nuevas."</p> }.into_any()
                        }
                        Some(lista) => view! {
                            <div>
                                {lista
                                    .into_iter()
                                    .map(|n| {
                                        let id = n.id;
                                        let hora = n.created_at.format("%d/%m %H:%M").to_string();
                                        view! {
                                            <button type="button" class="notif-item" on:click=move |_| marcar(id)>
                                                <h5>{n.title.clone()}</h5>
                                                <p>{n.body.clone()}</p>
                                                <p class="text-faint">{hora}</p>
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                            </div>
                        }
                        .into_any(),
                    }}
                </div>
            </Portal>
        </Show>
    }
}
