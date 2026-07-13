//! Pestaña "Previstos": pagos e ingresos futuros de una sola vez (backend
//! `planned_transactions`) — complementa a Suscripciones, que son
//! recurrentes. "Borrar" aquí sí es definitivo: no ajusta ningún saldo
//! (a diferencia de una transacción real) y el backend no tiene columna
//! de borrado lógico para esta tabla.

use chrono::NaiveDate;
use leptos::ev::SubmitEvent;
use leptos::portal::Portal;
use leptos::prelude::*;
use uuid::Uuid;

use super::util::{confirmar, hoy};
use crate::api::{accounting, accounts, agenda};
use crate::auth::{token_vigente, use_auth};
use crate::components::menu_flotante::{abrir_menu, estilo_posicion};

fn nombre_categoria(categorias: &[accounting::Categoria], id: Option<Uuid>) -> String {
    id.and_then(|id| categorias.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sin categoría".to_string())
}

fn nombre_cuenta(cuentas: &[accounts::Cuenta], id: Option<Uuid>) -> String {
    id.and_then(|id| cuentas.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "—".to_string())
}

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(agenda::Previsto),
}

#[component]
pub fn PestanaPrevistos(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let modo = RwSignal::new(ModoFormulario::Cerrado);

    let filtro_estado = RwSignal::new(String::new());
    let filtro_desde = RwSignal::new(String::new());
    let filtro_hasta = RwSignal::new(String::new());

    let categorias = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounting::listar_categorias(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let cuentas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounts::listar_cuentas(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let previstos = LocalResource::new(move || {
        let estado = filtro_estado.get();
        let desde = filtro_desde.get();
        let hasta = filtro_hasta.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let filtros = agenda::FiltrosPrevistos {
                desde: desde.parse().ok(),
                hasta: hasta.parse().ok(),
                pagado: match estado.as_str() {
                    "pagados" => Some(true),
                    "pendientes" => Some(false),
                    _ => None,
                },
            };
            agenda::listar_previstos(workspace_id, &filtros, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Previstos"</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| modo.set(ModoFormulario::Crear)
                >
                    "+ Nuevo previsto"
                </button>
            </div>

            <div style="display:flex; gap:12px; margin-bottom:16px; flex-wrap:wrap;">
                <select
                    style="max-width:170px;"
                    prop:value=move || filtro_estado.get()
                    on:change=move |ev| filtro_estado.set(event_target_value(&ev))
                >
                    <option value="">"Todos"</option>
                    <option value="pendientes">"Pendientes"</option>
                    <option value="pagados">"Pagados"</option>
                </select>
                <input
                    type="date"
                    prop:value=move || filtro_desde.get()
                    on:input=move |ev| filtro_desde.set(event_target_value(&ev))
                />
                <input
                    type="date"
                    prop:value=move || filtro_hasta.get()
                    on:input=move |ev| filtro_hasta.set(event_target_value(&ev))
                />
            </div>

            {move || match modo.get() {
                ModoFormulario::Cerrado => ().into_any(),
                ModoFormulario::Crear => view! {
                    <FormularioPrevisto
                        workspace_id=workspace_id
                        categorias=categorias.get().unwrap_or_default()
                        cuentas=cuentas.get().unwrap_or_default()
                        previsto_existente=None
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); previstos.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
                ModoFormulario::Editar(p) => view! {
                    <FormularioPrevisto
                        workspace_id=workspace_id
                        categorias=categorias.get().unwrap_or_default()
                        cuentas=cuentas.get().unwrap_or_default()
                        previsto_existente=Some(p)
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); previstos.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
            }}

            {move || {
                let categorias_ok = categorias.get().unwrap_or_default();
                let cuentas_ok = cuentas.get().unwrap_or_default();
                match previstos.get() {
                    None => view! { <p class="text-soft">"Cargando previstos..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok(lista)) if lista.is_empty() => {
                        view! { <p class="text-soft">"No hay previstos con estos filtros."</p> }.into_any()
                    }
                    Some(Ok(lista)) => view! {
                        <TablaPrevistos
                            workspace_id=workspace_id
                            lista=lista
                            categorias=categorias_ok
                            cuentas=cuentas_ok
                            on_editar=move |p| modo.set(ModoFormulario::Editar(p))
                            on_cambio=move || previstos.refetch()
                        />
                    }
                    .into_any(),
                }
            }}
        </section>
    }
}

#[component]
fn MenuPrevisto<FE, FP, FB>(
    pagado: bool,
    on_editar: FE,
    on_pagado: FP,
    on_borrar: FB,
) -> impl IntoView
where
    FE: Fn() + 'static + Copy + Send + Sync,
    FP: Fn() + 'static + Copy + Send + Sync,
    FB: Fn() + 'static + Copy + Send + Sync,
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
                        <button type="button" class="menu-item" on:click=move |_| { abierto.set(false); on_editar(); }>
                            "Editar"
                        </button>
                        <Show when=move || !pagado>
                            <button type="button" class="menu-item" on:click=move |_| { abierto.set(false); on_pagado(); }>
                                "Marcar pagado"
                            </button>
                        </Show>
                        <button type="button" class="menu-item is-danger" on:click=move |_| { abierto.set(false); on_borrar(); }>
                            "Borrar"
                        </button>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}

#[component]
fn TablaPrevistos<FE, FA>(
    workspace_id: Uuid,
    lista: Vec<agenda::Previsto>,
    categorias: Vec<accounting::Categoria>,
    cuentas: Vec<accounts::Cuenta>,
    on_editar: FE,
    on_cambio: FA,
) -> impl IntoView
where
    FE: Fn(agenda::Previsto) + 'static + Copy + Send + Sync,
    FA: Fn() + 'static + Copy + Send + Sync,
{
    let auth = use_auth();

    let marcar_pagado = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = agenda::marcar_pagado(workspace_id, id, &token).await;
                on_cambio();
            }
        });
    };

    let borrar = move |id: Uuid| {
        if !confirmar("¿Eliminar este previsto?") {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = agenda::eliminar_previsto(workspace_id, id, &token).await;
                on_cambio();
            }
        });
    };

    view! {
        <div class="table-scroll">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Vencimiento"</th>
                        <th>"Tipo"</th>
                        <th>"Descripción"</th>
                        <th>"Categoría"</th>
                        <th>"Cuenta"</th>
                        <th>"Monto"</th>
                        <th>"Estado"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {lista
                        .into_iter()
                        .map(|p| {
                            let guardado = StoredValue::new(p.clone());
                            let id = p.id;
                            let pagado = p.is_paid;
                            let signo = if p.tipo == "income" { "+" } else { "-" };
                            let color = if p.tipo == "income" { "var(--positive)" } else { "var(--negative)" };
                            view! {
                                <tr>
                                    <td>{p.due_date.to_string()}</td>
                                    <td>{if p.tipo == "income" { "Ingreso" } else { "Egreso" }}</td>
                                    <td>{p.description.clone().unwrap_or_else(|| "—".to_string())}</td>
                                    <td>{nombre_categoria(&categorias, p.category_id)}</td>
                                    <td>{nombre_cuenta(&cuentas, p.account_id)}</td>
                                    <td class="num" style=format!("color:{color};")>{format!("{signo}{:.2}", p.amount)}</td>
                                    <td>{if pagado { "Pagado" } else { "Pendiente" }}</td>
                                    <td>
                                        <MenuPrevisto
                                            pagado=pagado
                                            on_editar=move || on_editar(guardado.get_value())
                                            on_pagado=move || marcar_pagado(id)
                                            on_borrar=move || borrar(id)
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
fn FormularioPrevisto<F1, F2>(
    workspace_id: Uuid,
    categorias: Vec<accounting::Categoria>,
    cuentas: Vec<accounts::Cuenta>,
    previsto_existente: Option<agenda::Previsto>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let es_edicion = previsto_existente.is_some();
    let id_existente = previsto_existente.as_ref().map(|p| p.id);
    let categorias = RwSignal::new(categorias);

    let tipo = RwSignal::new(
        previsto_existente
            .as_ref()
            .map(|p| p.tipo.clone())
            .unwrap_or_else(|| "expense".to_string()),
    );
    let monto = RwSignal::new(
        previsto_existente
            .as_ref()
            .map(|p| p.amount.to_string())
            .unwrap_or_default(),
    );
    let vencimiento = RwSignal::new(
        previsto_existente
            .as_ref()
            .map(|p| p.due_date.to_string())
            .unwrap_or_else(|| hoy().to_string()),
    );
    let categoria_id = RwSignal::new(
        previsto_existente
            .as_ref()
            .and_then(|p| p.category_id)
            .map(|id| id.to_string())
            .unwrap_or_default(),
    );
    let cuenta_id = RwSignal::new(
        previsto_existente
            .as_ref()
            .and_then(|p| p.account_id)
            .map(|id| id.to_string())
            .unwrap_or_default(),
    );
    let descripcion = RwSignal::new(
        previsto_existente
            .as_ref()
            .and_then(|p| p.description.clone())
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let categorias_filtradas = move || {
        let tipo_actual = tipo.get();
        categorias
            .get()
            .into_iter()
            .filter(|c| c.tipo == tipo_actual)
            .collect::<Vec<_>>()
    };

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };
        let Ok(due_date) = vencimiento.get_untracked().parse::<NaiveDate>() else {
            error.set(Some("La fecha de vencimiento no es válida".to_string()));
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

            let tipo_actual = tipo.get_untracked();
            let descripcion_actual = descripcion.get_untracked();
            let datos = agenda::DatosPrevisto {
                tipo: &tipo_actual,
                amount,
                due_date,
                category_id,
                account_id,
                description: if descripcion_actual.trim().is_empty() {
                    None
                } else {
                    Some(descripcion_actual.trim())
                },
            };

            let resultado = if let Some(id) = id_existente {
                agenda::actualizar_previsto(workspace_id, id, &datos, &token)
                    .await
                    .map(|_| ())
            } else {
                agenda::crear_previsto(workspace_id, &datos, &token)
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
                    <label>"Tipo"</label>
                    <select
                        prop:value=move || tipo.get()
                        on:change=move |ev| {
                            tipo.set(event_target_value(&ev));
                            // La categoría elegida puede quedar de un tipo que ya
                            // no coincide con el nuevo Tipo (el backend valida
                            // que sean del mismo tipo) — se limpia para no
                            // mandar una combinación inválida sin que el usuario
                            // se dé cuenta.
                            categoria_id.set(String::new());
                        }
                    >
                        <option value="expense">"Egreso"</option>
                        <option value="income">"Ingreso"</option>
                    </select>
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
                    <label>"Fecha de vencimiento"</label>
                    <input
                        type="date"
                        prop:value=move || vencimiento.get()
                        on:input=move |ev| vencimiento.set(event_target_value(&ev))
                    />
                </div>
                <div class="field">
                    <label>"Categoría"</label>
                    <select
                        prop:value=move || categoria_id.get()
                        on:change=move |ev| categoria_id.set(event_target_value(&ev))
                    >
                        <option value="">"Sin categoría"</option>
                        {move || {
                            categorias_filtradas()
                                .into_iter()
                                .map(|c| {
                                    let id = c.id.to_string();
                                    view! { <option value=id.clone()>{c.name.clone()}</option> }
                                })
                                .collect_view()
                        }}
                    </select>
                </div>
                <div class="field">
                    <label>"Cuenta (opcional)"</label>
                    <select
                        prop:value=move || cuenta_id.get()
                        on:change=move |ev| cuenta_id.set(event_target_value(&ev))
                    >
                        <option value="">"Sin cuenta"</option>
                        {cuentas
                            .iter()
                            .map(|c| {
                                let id = c.id.to_string();
                                view! { <option value=id.clone()>{c.name.clone()}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
                <div class="field">
                    <label>"Descripción"</label>
                    <input
                        prop:value=move || descripcion.get()
                        on:input=move |ev| descripcion.set(event_target_value(&ev))
                    />
                </div>
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
                            "Crear previsto"
                        }
                    }}
                </button>
            </div>
        </form>
    }
}
