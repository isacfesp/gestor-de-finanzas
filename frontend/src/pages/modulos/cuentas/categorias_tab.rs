//! Pestaña "Categorías y Etiquetas": gestión de la taxonomía que usan las
//! transacciones (categorías, agrupadas por su tipo/funcionalidad —
//! Ingreso o Gasto — y etiquetas), separada de Transacciones para que no
//! se definan al vuelo dentro del formulario de un movimiento.

use leptos::prelude::*;
use uuid::Uuid;

use crate::api::{accounting, tags};
use crate::auth::{token_vigente, use_auth};

#[component]
pub fn PestanaCategorias(workspace_id: Uuid) -> impl IntoView {
    view! {
        <SeccionCategorias workspace_id=workspace_id/>
        <SeccionEtiquetas workspace_id=workspace_id/>
    }
}

#[component]
fn SeccionCategorias(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let categorias = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounting::listar_categorias(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let nombre = RwSignal::new(String::new());
    let tipo = RwSignal::new("expense".to_string());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let crear = move |_| {
        let valor = nombre.get_untracked();
        if valor.trim().is_empty() || guardando.get_untracked() {
            return;
        }
        error.set(None);
        guardando.set(true);
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let datos = accounting::CrearCategoriaDatos {
                    name: valor.trim(),
                    tipo: &tipo.get_untracked(),
                };
                match accounting::crear_categoria(workspace_id, &datos, &token).await {
                    Ok(_) => {
                        categorias.refetch();
                        nombre.set(String::new());
                    }
                    Err(e) => error.set(Some(e.to_string())),
                }
            }
            guardando.set(false);
        });
    };

    let eliminar = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                match accounting::eliminar_categoria(workspace_id, id, &token).await {
                    Ok(()) => categorias.refetch(),
                    Err(e) => error.set(Some(e.to_string())),
                }
            }
        });
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Categorías"</h2>
            </div>

            <div class="form-grid" style="margin-bottom:16px;">
                <div class="field">
                    <label>"Nombre"</label>
                    <input
                        prop:value=move || nombre.get()
                        on:input=move |ev| nombre.set(event_target_value(&ev))
                    />
                </div>
                <div class="field">
                    <label>"Tipo"</label>
                    <select prop:value=move || tipo.get() on:change=move |ev| tipo.set(event_target_value(&ev))>
                        <option value="expense">"Gasto"</option>
                        <option value="income">"Ingreso"</option>
                    </select>
                </div>
                <div class="field" style="justify-content:flex-end;">
                    <button type="button" class="btn btn-primary" disabled=move || guardando.get() on:click=crear>
                        {move || if guardando.get() { "Creando..." } else { "+ Crear categoría" }}
                    </button>
                </div>
            </div>

            <Show when=move || error.get().is_some()>
                <p class="banner banner-error" style="margin-bottom:14px;">
                    {move || error.get().unwrap_or_default()}
                </p>
            </Show>

            {move || match categorias.get() {
                None => view! { <p class="text-soft">"Cargando categorías..."</p> }.into_any(),
                Some(lista) => {
                    let ingreso = lista.iter().filter(|c| c.tipo == "income").cloned().collect::<Vec<_>>();
                    let gasto = lista.iter().filter(|c| c.tipo == "expense").cloned().collect::<Vec<_>>();
                    view! {
                        <ListaCategorias titulo="Ingreso" lista=ingreso on_eliminar=eliminar/>
                        <ListaCategorias titulo="Gasto" lista=gasto on_eliminar=eliminar/>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

#[component]
fn ListaCategorias<F>(
    titulo: &'static str,
    lista: Vec<accounting::Categoria>,
    on_eliminar: F,
) -> impl IntoView
where
    F: Fn(Uuid) + 'static + Copy,
{
    view! {
        <div style="margin-bottom:16px;">
            <p class="text-soft" style="font-size:12.5px; font-weight:700; text-transform:uppercase; margin-bottom:8px;">
                {titulo}
            </p>
            {if lista.is_empty() {
                view! { <p class="text-faint" style="font-size:13px;">"Sin categorías."</p> }.into_any()
            } else {
                view! {
                    <div style="display:flex; gap:8px; flex-wrap:wrap;">
                        {lista
                            .into_iter()
                            .map(|c| {
                                let es_propia = c.workspace_id.is_some();
                                let id = c.id;
                                view! {
                                    <span class="chip">
                                        {c.name.clone()}
                                        {if es_propia {
                                            view! {
                                                <button
                                                    type="button"
                                                    style="color:var(--negative); font-weight:800; line-height:1;"
                                                    title="Eliminar"
                                                    on:click=move |_| on_eliminar(id)
                                                >
                                                    "×"
                                                </button>
                                            }
                                            .into_any()
                                        } else {
                                            view! { <span class="text-faint" style="font-size:11px;">"(global)"</span> }
                                                .into_any()
                                        }}
                                    </span>
                                }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any()
            }}
        </div>
    }
}

#[component]
fn SeccionEtiquetas(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();

    let etiquetas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        tags::listar_etiquetas(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let nueva = RwSignal::new(String::new());
    let creando = RwSignal::new(false);

    let crear = move |_| {
        let nombre = nueva.get_untracked();
        if nombre.trim().is_empty() || creando.get_untracked() {
            return;
        }
        creando.set(true);
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let datos = tags::CrearEtiquetaDatos {
                    name: nombre.trim(),
                };
                if tags::crear_etiqueta(workspace_id, &datos, &token)
                    .await
                    .is_ok()
                {
                    etiquetas.refetch();
                    nueva.set(String::new());
                }
            }
            creando.set(false);
        });
    };

    let desactivar = move |id: Uuid| {
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = tags::desactivar_etiqueta(workspace_id, id, &token).await;
                etiquetas.refetch();
            }
        });
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Etiquetas"</h2>
            </div>
            <div style="display:flex; gap:8px; flex-wrap:wrap; margin-bottom:14px;">
                {move || {
                    etiquetas
                        .get()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|etiqueta| {
                            let id = etiqueta.id;
                            view! {
                                <span class="chip">
                                    {etiqueta.name}
                                    <button
                                        type="button"
                                        style="color:var(--negative); font-weight:800; line-height:1;"
                                        title="Desactivar"
                                        on:click=move |_| desactivar(id)
                                    >
                                        "×"
                                    </button>
                                </span>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div style="display:flex; gap:6px; max-width:320px;">
                <input
                    placeholder="Nueva etiqueta..."
                    style="flex:1;"
                    prop:value=move || nueva.get()
                    on:input=move |ev| nueva.set(event_target_value(&ev))
                />
                <button type="button" class="btn-ghost" on:click=crear>
                    "+"
                </button>
            </div>
        </section>
    }
}
