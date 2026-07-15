//! Pestaña "Suscripciones": gastos fijos recurrentes (backend
//! `accounting::suscripciones`). No hay borrado: se activan/desactivan
//! (mismo criterio que Cuentas) para no perder el historial de cobros ya
//! marcados.

use leptos::ev::SubmitEvent;
use leptos::portal::Portal;
use leptos::prelude::*;
use uuid::Uuid;

use super::util::hoy;
use crate::api::{accounting, accounts, agenda};
use crate::auth::{token_vigente, use_auth};
use crate::components::hoja_inferior::HojaInferior;
use crate::components::menu_flotante::{abrir_menu, estilo_posicion};
use crate::pages::modulos::cuentas::{FormularioCategoria, FormularioCuenta};
use crate::workspace::use_workspace;

fn nombre_categoria(categorias: &[accounting::Categoria], id: Option<Uuid>) -> String {
    id.and_then(|id| categorias.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sin categoría".to_string())
}

/// Las cuentas son personales: el formulario de suscripción solo puede
/// asignar una cuenta propia (el backend lo exige igual, esto solo
/// evita ofrecer opciones que el servidor rechazaría).
fn cuentas_propias(cuentas: &[accounts::Cuenta], mi_id: Option<Uuid>) -> Vec<accounts::Cuenta> {
    cuentas
        .iter()
        .filter(|c| Some(c.owner_id) == mi_id)
        .cloned()
        .collect()
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
    let workspace = use_workspace();
    let modo = RwSignal::new(ModoFormulario::Cerrado);
    let mi_id = move || auth.usuario().map(|u| u.id);

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

    let cuentas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounts::listar_cuentas(workspace_id, &token)
            .await
            .unwrap_or_default()
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
                        cuentas=cuentas_propias(&cuentas.get().unwrap_or_default(), mi_id())
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
                        cuentas=cuentas_propias(&cuentas.get().unwrap_or_default(), mi_id())
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
                    Some(Ok(lista)) => {
                        let (propias, ajenas): (Vec<_>, Vec<_>) = lista
                            .into_iter()
                            .partition(|s| Some(s.owner_id) == mi_id());
                        let hay_ajenas = !ajenas.is_empty();
                        let categorias_ajenas = categorias_ok.clone();
                        view! {
                            {if propias.is_empty() {
                                view! { <p class="text-soft">"Todavía no tienes suscripciones."</p> }.into_any()
                            } else {
                                view! {
                                    <TablaSuscripciones
                                        workspace_id=workspace_id
                                        lista=propias
                                        categorias=categorias_ok
                                        editable=true
                                        on_editar=move |s| modo.set(ModoFormulario::Editar(s))
                                        on_cambio=move || suscripciones.refetch()
                                    />
                                }
                                .into_any()
                            }}
                            <Show when=move || workspace.puede_supervisar() && hay_ajenas>
                                <h3 class="text-soft" style="margin:24px 0 12px; font-size:13px; font-weight:600;">
                                    "Suscripciones de otros miembros (supervisión)"
                                </h3>
                                <TablaSuscripciones
                                    workspace_id=workspace_id
                                    lista=ajenas.clone()
                                    categorias=categorias_ajenas.clone()
                                    editable=false
                                    on_editar=move |s| modo.set(ModoFormulario::Editar(s))
                                    on_cambio=move || suscripciones.refetch()
                                />
                            </Show>
                        }
                        .into_any()
                    }
                }
            }}
        </section>
    }
}

#[component]
fn MenuSuscripcion<FE, FC, FT>(
    activa: bool,
    editable: bool,
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
                abrir_menu(ev, abierto, posicion, 180.0)
            }>
                <svg viewBox="0 0 24 24">
                    <circle cx="12" cy="12" r="3"></circle>
                    <path d="M12 2v3M12 19v3M4.2 4.2l2.1 2.1M17.7 17.7l2.1 2.1M2 12h3M19 12h3M4.2 19.8l2.1-2.1M17.7 6.3l2.1-2.1"></path>
                </svg>
            </button>
            <Show when=move || abierto.get()>
                <Portal>
                    <div class="menu-dropdown" style=move || estilo_posicion(posicion) on:mouseleave=move |_| abierto.set(false)>
                        <Show when=move || editable>
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
                        </Show>
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
    editable: bool,
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
                    account_id: s.account_id,
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
                                            editable=editable
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
    cuentas: Vec<accounts::Cuenta>,
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
    // Signal (no solo prop estática) para poder agregar la categoría/
    // cuenta recién creada desde la hoja de creación rápida.
    let categorias = RwSignal::new(categorias);
    let cuentas = RwSignal::new(cuentas);

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
    let cuenta_id = RwSignal::new(
        suscripcion_existente
            .as_ref()
            .and_then(|s| s.account_id)
            .map(|id| id.to_string())
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);
    let mostrar_categoria_rapida = RwSignal::new(false);
    let mostrar_cuenta_rapida = RwSignal::new(false);

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
        let account_id = Uuid::parse_str(&cuenta_id.get_untracked()).ok();

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
                        account_id,
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
                        account_id,
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
                        inputmode="decimal"
                        prop:value=move || monto.get()
                        on:input=move |ev| monto.set(event_target_value(&ev))
                    />
                </div>
                <div class="field">
                    <label>"Categoría"</label>
                    <div style="display:flex; gap:6px;">
                        <select
                            style="flex:1;"
                            prop:value=move || categoria_id.get()
                            on:change=move |ev| categoria_id.set(event_target_value(&ev))
                        >
                            <option value="">"Sin categoría"</option>
                            {move || {
                                categorias
                                    .get()
                                    .into_iter()
                                    .map(|c| {
                                        let id = c.id.to_string();
                                        view! { <option value=id.clone()>{c.name.clone()}</option> }
                                    })
                                    .collect_view()
                            }}
                        </select>
                        <button
                            type="button"
                            class="btn-ghost"
                            title="Crear categoría"
                            on:click=move |_| mostrar_categoria_rapida.set(true)
                        >
                            "+"
                        </button>
                    </div>
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
                <div class="field">
                    <label>"Cuenta (opcional)"</label>
                    <div style="display:flex; gap:6px;">
                        <select
                            style="flex:1;"
                            prop:value=move || cuenta_id.get()
                            on:change=move |ev| cuenta_id.set(event_target_value(&ev))
                        >
                            <option value="">"Sin cuenta"</option>
                            {move || {
                                cuentas
                                    .get()
                                    .into_iter()
                                    .map(|c| {
                                        let id = c.id.to_string();
                                        view! { <option value=id.clone()>{c.name.clone()}</option> }
                                    })
                                    .collect_view()
                            }}
                        </select>
                        <button
                            type="button"
                            class="btn-ghost"
                            title="Crear cuenta"
                            on:click=move |_| mostrar_cuenta_rapida.set(true)
                        >
                            "+"
                        </button>
                    </div>
                </div>

                <HojaInferior abierto=mostrar_categoria_rapida>
                    <FormularioCategoria
                        workspace_id=workspace_id
                        tipo_inicial="expense"
                        on_creada=move |creada| {
                            categoria_id.set(creada.id.to_string());
                            categorias.update(|lista| lista.push(creada));
                            mostrar_categoria_rapida.set(false);
                        }
                        on_cancelar=move || mostrar_categoria_rapida.set(false)
                    />
                </HojaInferior>
                <HojaInferior abierto=mostrar_cuenta_rapida>
                    <FormularioCuenta
                        workspace_id=workspace_id
                        cuenta_existente=None
                        on_guardado=move |creada| {
                            if let Some(creada) = creada {
                                cuenta_id.set(creada.id.to_string());
                                cuentas.update(|lista| lista.push(creada));
                            }
                            mostrar_cuenta_rapida.set(false);
                        }
                        on_cancelar=move || mostrar_cuenta_rapida.set(false)
                    />
                </HojaInferior>
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
