//! Pestaña "Suscripciones": gastos fijos recurrentes (backend
//! `accounting::suscripciones`). No hay borrado: se activan/desactivan
//! (mismo criterio que Cuentas) para no perder el historial de cobros ya
//! marcados.

use leptos::ev::SubmitEvent;
use leptos::portal::Portal;
use leptos::prelude::*;
use uuid::Uuid;

use super::util::hoy;
use crate::api::{accounting, agenda};
use crate::auth::{token_vigente, use_auth};
use crate::components::menu_flotante::{abrir_menu, estilo_posicion};

fn nombre_categoria(categorias: &[accounting::Categoria], id: Option<Uuid>) -> String {
    id.and_then(|id| categorias.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sin categoría".to_string())
}

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(agenda::Suscripcion),
}

#[component]
pub fn PestanaSuscripciones(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let modo = RwSignal::new(ModoFormulario::Cerrado);

    let categorias = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounting::listar_categorias(workspace_id, &token)
            .await
            .unwrap_or_default()
            .into_iter()
            .filter(|c| c.tipo == "expense")
            .collect::<Vec<_>>()
    });

    let suscripciones = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        agenda::listar_suscripciones(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Suscripciones"</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| modo.set(ModoFormulario::Crear)
                >
                    "+ Nueva suscripción"
                </button>
            </div>

            {move || match modo.get() {
                ModoFormulario::Cerrado => ().into_any(),
                ModoFormulario::Crear => view! {
                    <FormularioSuscripcion
                        workspace_id=workspace_id
                        categorias=categorias.get().unwrap_or_default()
                        suscripcion_existente=None
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); suscripciones.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
                ModoFormulario::Editar(s) => view! {
                    <FormularioSuscripcion
                        workspace_id=workspace_id
                        categorias=categorias.get().unwrap_or_default()
                        suscripcion_existente=Some(s)
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); suscripciones.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
            }}

            {move || {
                let categorias_ok = categorias.get().unwrap_or_default();
                match suscripciones.get() {
                    None => view! { <p class="text-soft">"Cargando suscripciones..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok(lista)) if lista.is_empty() => {
                        view! { <p class="text-soft">"Todavía no hay suscripciones."</p> }.into_any()
                    }
                    Some(Ok(lista)) => view! {
                        <TablaSuscripciones
                            workspace_id=workspace_id
                            lista=lista
                            categorias=categorias_ok
                            on_editar=move |s| modo.set(ModoFormulario::Editar(s))
                            on_cambio=move || suscripciones.refetch()
                        />
                    }
                    .into_any(),
                }
            }}
        </section>
    }
}

#[component]
fn MenuSuscripcion<FE, FC, FT>(
    activa: bool,
    on_editar: FE,
    on_cobrada: FC,
    on_activar: FT,
) -> impl IntoView
where
    FE: Fn() + 'static + Copy + Send + Sync,
    FC: Fn() + 'static + Copy + Send + Sync,
    FT: Fn() + 'static + Copy + Send + Sync,
{
    let abierto = RwSignal::new(false);
    let posicion = RwSignal::new((0.0_f64, 0.0_f64));

    view! {
        <div class="menu-gear">
            <button type="button" class="menu-gear-btn" title="Acciones" on:click=move |ev| {
                abrir_menu(ev, abierto, posicion)
            }>
                <svg viewBox="0 0 24 24">
                    <circle cx="12" cy="12" r="3"></circle>
                    <path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"></path>
                </svg>
            </button>
            <Show when=move || abierto.get()>
                <Portal>
                    <div class="menu-dropdown" style=move || estilo_posicion(posicion) on:mouseleave=move |_| abierto.set(false)>
                        <button type="button" class="menu-item" on:click=move |_| { abierto.set(false); on_editar(); }>
                            "Editar"
                        </button>
                        <Show when=move || activa>
                            <button type="button" class="menu-item" on:click=move |_| { abierto.set(false); on_cobrada(); }>
                                "Marcar cobrada"
                            </button>
                        </Show>
                        <button
                            type="button"
                            class=if activa { "menu-item is-danger" } else { "menu-item" }
                            on:click=move |_| { abierto.set(false); on_activar(); }
                        >
                            {if activa { "Desactivar" } else { "Activar" }}
                        </button>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}

#[component]
fn TablaSuscripciones<FE, FA>(
    workspace_id: Uuid,
    lista: Vec<agenda::Suscripcion>,
    categorias: Vec<accounting::Categoria>,
    on_editar: FE,
    on_cambio: FA,
) -> impl IntoView
where
    FE: Fn(agenda::Suscripcion) + 'static + Copy + Send + Sync,
    FA: Fn() + 'static + Copy + Send + Sync,
{
    let auth = use_auth();

    let marcar_cobrada = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = agenda::marcar_cobrada(workspace_id, id, &token).await;
                on_cambio();
            }
        });
    };

    let alternar_activa = move |s: agenda::Suscripcion| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let datos = agenda::ActualizarSuscripcionDatos {
                    name: &s.name,
                    amount: s.amount,
                    category_id: s.category_id,
                    periodicity: &s.periodicity,
                    next_billing_date: s.next_billing_date,
                    is_active: !s.is_active,
                };
                let _ = agenda::actualizar_suscripcion(workspace_id, s.id, &datos, &token).await;
                on_cambio();
            }
        });
    };

    view! {
        <div class="table-scroll">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Nombre"</th>
                        <th>"Monto"</th>
                        <th>"Categoría"</th>
                        <th>"Periodicidad"</th>
                        <th>"Próximo cobro"</th>
                        <th>"Estado"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {lista
                        .into_iter()
                        .map(|s| {
                            let guardada = StoredValue::new(s.clone());
                            let activa = s.is_active;
                            let id = s.id;
                            view! {
                                <tr>
                                    <td>{s.name.clone()}</td>
                                    <td class="num">{format!("{:.2}", s.amount)}</td>
                                    <td>{nombre_categoria(&categorias, s.category_id)}</td>
                                    <td>{agenda::etiqueta_periodicidad(&s.periodicity)}</td>
                                    <td>{s.next_billing_date.to_string()}</td>
                                    <td>{if activa { "Activa" } else { "Inactiva" }}</td>
                                    <td>
                                        <MenuSuscripcion
                                            activa=activa
                                            on_editar=move || on_editar(guardada.get_value())
                                            on_cobrada=move || marcar_cobrada(id)
                                            on_activar=move || alternar_activa(guardada.get_value())
                                        />
                                    </td>
                                </tr>
                            }
                        })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn FormularioSuscripcion<F1, F2>(
    workspace_id: Uuid,
    categorias: Vec<accounting::Categoria>,
    suscripcion_existente: Option<agenda::Suscripcion>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let es_edicion = suscripcion_existente.is_some();
    let id_existente = suscripcion_existente.as_ref().map(|s| s.id);

    let nombre = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_default(),
    );
    let monto = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .map(|s| s.amount.to_string())
            .unwrap_or_default(),
    );
    let categoria_id = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .and_then(|s| s.category_id)
            .map(|id| id.to_string())
            .unwrap_or_default(),
    );
    let periodicidad = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .map(|s| s.periodicity.clone())
            .unwrap_or_else(|| "monthly".to_string()),
    );
    let proximo_cobro = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .map(|s| s.next_billing_date.to_string())
            .unwrap_or_else(|| hoy().to_string()),
    );
    let activa = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .map(|s| s.is_active)
            .unwrap_or(true),
    );
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        if nombre.get_untracked().trim().is_empty() {
            error.set(Some("El nombre no puede estar vacío".to_string()));
            return;
        }
        let Ok(amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };
        let Ok(next_billing_date) = proximo_cobro.get_untracked().parse() else {
            error.set(Some("La fecha de próximo cobro no es válida".to_string()));
            return;
        };
        let category_id = Uuid::parse_str(&categoria_id.get_untracked()).ok();

        guardando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let periodicity = periodicidad.get_untracked();
            let resultado = if let Some(id) = id_existente {
                agenda::actualizar_suscripcion(
                    workspace_id,
                    id,
                    &agenda::ActualizarSuscripcionDatos {
                        name: &nombre.get_untracked(),
                        amount,
                        category_id,
                        periodicity: &periodicity,
                        next_billing_date,
                        is_active: activa.get_untracked(),
                    },
                    &token,
                )
                .await
                .map(|_| ())
            } else {
                agenda::crear_suscripcion(
                    workspace_id,
                    &agenda::DatosSuscripcion {
                        name: &nombre.get_untracked(),
                        amount,
                        category_id,
                        periodicity: &periodicity,
                        next_billing_date,
                    },
                    &token,
                )
                .await
                .map(|_| ())
            };

            guardando.set(false);
            match resultado {
                Ok(()) => on_guardado(),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Nombre"</label>
                    <input
                        prop:value=move || nombre.get()
                        on:input=move |ev| nombre.set(event_target_value(&ev))
                        required
                    />
                </div>
                <div class="field">
                    <label>"Monto"</label>
                    <input
                        placeholder="0.00"
                        prop:value=move || monto.get()
                        on:input=move |ev| monto.set(event_target_value(&ev))
                    />
                </div>
                <div class="field">
                    <label>"Categoría"</label>
                    <select
                        prop:value=move || categoria_id.get()
                        on:change=move |ev| categoria_id.set(event_target_value(&ev))
                    >
                        <option value="">"Sin categoría"</option>
                        {categorias
                            .iter()
                            .map(|c| {
                                let id = c.id.to_string();
                                view! { <option value=id.clone()>{c.name.clone()}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
                <div class="field">
                    <label>"Periodicidad"</label>
                    <select
                        prop:value=move || periodicidad.get()
                        on:change=move |ev| periodicidad.set(event_target_value(&ev))
                    >
                        {agenda::PERIODICIDADES
                            .iter()
                            .map(|(valor, etiqueta)| view! { <option value=*valor>{*etiqueta}</option> })
                            .collect_view()}
                    </select>
                </div>
                <div class="field">
                    <label>"Próximo cobro"</label>
                    <input
                        type="date"
                        prop:value=move || proximo_cobro.get()
                        on:input=move |ev| proximo_cobro.set(event_target_value(&ev))
                    />
                </div>
                <Show when=move || es_edicion>
                    <div class="field">
                        <label>"Estado"</label>
                        <select
                            prop:value=move || if activa.get() { "true" } else { "false" }
                            on:change=move |ev| activa.set(event_target_value(&ev) == "true")
                        >
                            <option value="true">"Activa"</option>
                            <option value="false">"Inactiva"</option>
                        </select>
                    </div>
                </Show>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="banner banner-error" style="margin-bottom:14px;">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            <div class="form-actions">
                <button type="button" class="btn-ghost" on:click=move |_| on_cancelar()>
                    "Cancelar"
                </button>
                <button type="submit" class="btn btn-primary" disabled=move || guardando.get()>
                    {move || {
                        if guardando.get() {
                            "Guardando..."
                        } else if es_edicion {
                            "Guardar cambios"
                        } else {
                            "Crear suscripción"
                        }
                    }}
                </button>
            </div>
        </form>
    }
}
