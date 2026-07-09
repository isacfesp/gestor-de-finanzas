//! Sección "Movimientos" de `docs/frontend-ia.md`: auditoría — historial
//! de actividad del workspace (quién hizo qué y cuándo). **No** son las
//! transacciones de ingresos/gastos, esas viven en Cuentas.
//!
//! Lee `GET /workspaces/:workspace_id/movimientos` (backend
//! `movimientos`), con alcance de workspace — a diferencia de
//! `GET /admin/auditoria`, que es global y solo para el rol dev.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::movimientos::{self, Movimiento, etiqueta_accion};
use crate::auth::{token_vigente, use_auth};
use crate::workspace::use_workspace;

const TAMANO_PAGINA: i64 = 50;

#[component]
pub fn MovimientosPage() -> impl IntoView {
    let workspace = use_workspace();

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
            <ListaMovimientos workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
        </Show>
    }
}

#[component]
fn ListaMovimientos(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let paginas_cargadas = RwSignal::new(1i64);
    let acumulado = RwSignal::new(Vec::<Movimiento>::new());
    let error = RwSignal::new(None::<String>);
    let cargando = RwSignal::new(false);
    let agotado = RwSignal::new(false);

    let cargar_pagina = move |desplazamiento: i64| {
        cargando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                cargando.set(false);
                return;
            };
            match movimientos::listar_movimientos(
                workspace_id,
                Some(TAMANO_PAGINA),
                Some(desplazamiento),
                &token,
            )
            .await
            {
                Ok(lista) => {
                    if lista.len() < TAMANO_PAGINA as usize {
                        agotado.set(true);
                    }
                    acumulado.update(|actuales| actuales.extend(lista));
                }
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
            cargando.set(false);
        });
    };

    // Primera carga.
    Effect::new(move |_| cargar_pagina(0));

    let cargar_mas = move |_| {
        let siguiente = paginas_cargadas.get();
        paginas_cargadas.set(siguiente + 1);
        cargar_pagina(siguiente * TAMANO_PAGINA);
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Movimientos"</h2>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="banner banner-error" style="margin-bottom:14px;">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            {move || {
                let lista = acumulado.get();
                if lista.is_empty() && !cargando.get() {
                    view! { <p class="text-soft">"Todavía no hay actividad registrada en este workspace."</p> }.into_any()
                } else {
                    view! {
                        <div class="table-scroll">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Fecha"</th>
                                        <th>"Quién"</th>
                                        <th>"Qué"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {lista
                                        .into_iter()
                                        .map(|m| {
                                            let detalle = m.detail.as_ref().map(|d| d.to_string()).unwrap_or_default();
                                            view! {
                                                <tr>
                                                    <td>{m.created_at.format("%d/%m/%Y %H:%M").to_string()}</td>
                                                    <td>{m.actor_name.clone()}</td>
                                                    <td title=detalle>{etiqueta_accion(&m.action).to_string()}</td>
                                                </tr>
                                            }
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }
                    .into_any()
                }
            }}

            <Show when=move || !agotado.get() && !acumulado.get().is_empty()>
                <div style="text-align:center; margin-top:14px;">
                    <button class="btn-ghost" on:click=cargar_mas disabled=move || cargando.get()>
                        {move || if cargando.get() { "Cargando..." } else { "Cargar más" }}
                    </button>
                </div>
            </Show>
        </section>
    }
}
