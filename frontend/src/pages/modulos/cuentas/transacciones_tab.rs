//! Pestaña "Transacciones": filtros, tabla y alta/edición de
//! ingresos/gastos, con selección de etiquetas al crear.

use leptos::prelude::*;
use uuid::Uuid;

use super::util::{confirmar, hoy};
use crate::api::{accounting, accounts, tags};
use crate::auth::{token_vigente, use_auth};

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(accounting::Transaccion),
}

#[component]
pub fn PestanaTransacciones(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let modo = RwSignal::new(ModoFormulario::Cerrado);

    let filtro_tipo = RwSignal::new(String::new());
    let filtro_desde = RwSignal::new(String::new());
    let filtro_hasta = RwSignal::new(String::new());

    let cuentas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounts::listar_cuentas(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let categorias = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounting::listar_categorias(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let transacciones = LocalResource::new(move || {
        let tipo = filtro_tipo.get();
        let desde = filtro_desde.get();
        let hasta = filtro_hasta.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let filtros = accounting::FiltrosTransacciones {
                tipo: if tipo.is_empty() {
                    None
                } else {
                    Some(tipo.as_str())
                },
                category_id: None,
                desde: desde.parse().ok(),
                hasta: hasta.parse().ok(),
            };
            accounting::listar_transacciones(workspace_id, &filtros, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Transacciones"</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| modo.set(ModoFormulario::Crear)
                >
                    "+ Nuevo movimiento"
                </button>
            </div>

            <div style="display:flex; gap:12px; margin-bottom:16px; flex-wrap:wrap;">
                <select
                    style="max-width:160px;"
                    prop:value=move || filtro_tipo.get()
                    on:change=move |ev| filtro_tipo.set(event_target_value(&ev))
                >
                    <option value="">"Todos"</option>
                    <option value="income">"Ingresos"</option>
                    <option value="expense">"Gastos"</option>
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
                    <FormularioTransaccion
                        workspace_id=workspace_id
                        cuentas=cuentas.get().unwrap_or_default()
                        categorias=categorias.get().unwrap_or_default()
                        transaccion_existente=None
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); transacciones.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
                ModoFormulario::Editar(t) => view! {
                    <FormularioTransaccion
                        workspace_id=workspace_id
                        cuentas=cuentas.get().unwrap_or_default()
                        categorias=categorias.get().unwrap_or_default()
                        transaccion_existente=Some(t)
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); transacciones.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
            }}

            {move || match transacciones.get() {
                None => view! { <p class="text-soft">"Cargando movimientos..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"No hay movimientos con estos filtros."</p> }.into_any()
                }
                Some(Ok(lista)) => {
                    let cuentas_lista = cuentas.get().unwrap_or_default();
                    let categorias_lista = categorias.get().unwrap_or_default();
                    view! {
                        <TablaTransacciones
                            workspace_id=workspace_id
                            transacciones=lista
                            cuentas=cuentas_lista
                            categorias=categorias_lista
                            on_editar=move |t| modo.set(ModoFormulario::Editar(t))
                            on_borrada=move || transacciones.refetch()
                        />
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

fn nombre_cuenta(cuentas: &[accounts::Cuenta], id: Uuid) -> String {
    cuentas
        .iter()
        .find(|c| c.id == id)
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "—".to_string())
}

fn nombre_categoria(categorias: &[accounting::Categoria], id: Option<Uuid>) -> String {
    id.and_then(|id| categorias.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sin categoría".to_string())
}

#[component]
fn TablaTransacciones<FE, FB>(
    workspace_id: Uuid,
    transacciones: Vec<accounting::Transaccion>,
    cuentas: Vec<accounts::Cuenta>,
    categorias: Vec<accounting::Categoria>,
    on_editar: FE,
    on_borrada: FB,
) -> impl IntoView
where
    FE: Fn(accounting::Transaccion) + 'static + Copy,
    FB: Fn() + 'static + Copy,
{
    let auth = use_auth();

    let borrar = move |id: Uuid| {
        if !confirmar("¿Eliminar este movimiento? Se ajustará el saldo de la cuenta.") {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = accounting::eliminar_transaccion(workspace_id, id, &token).await;
                on_borrada();
            }
        });
    };

    view! {
        <table class="data-table">
            <thead>
                <tr>
                    <th>"Fecha"</th>
                    <th>"Descripción"</th>
                    <th>"Categoría"</th>
                    <th>"Cuenta"</th>
                    <th>"Monto"</th>
                    <th></th>
                </tr>
            </thead>
            <tbody>
                {transacciones
                    .into_iter()
                    .map(|t| {
                        let color = if t.tipo == "income" { "var(--positive)" } else { "var(--negative)" };
                        let signo = if t.tipo == "income" { "+" } else { "-" };
                        let para_editar = t.clone();
                        let id = t.id;
                        view! {
                            <tr>
                                <td>{t.date.to_string()}</td>
                                <td>{t.description.clone().unwrap_or_else(|| "—".to_string())}</td>
                                <td>{nombre_categoria(&categorias, t.category_id)}</td>
                                <td>{nombre_cuenta(&cuentas, t.account_id)}</td>
                                <td class="num" style=format!("color:{color};")>{format!("{signo}{:.2}", t.amount)}</td>
                                <td>
                                    <div class="row-actions">
                                        <button class="btn-ghost" style="padding:4px 8px;" on:click=move |_| on_editar(para_editar.clone())>
                                            "Editar"
                                        </button>
                                        <button class="btn-ghost" style="padding:4px 8px;" on:click=move |_| borrar(id)>
                                            "Borrar"
                                        </button>
                                    </div>
                                </td>
                            </tr>
                        }
                    })
                    .collect_view()}
            </tbody>
        </table>
    }
}

#[component]
fn FormularioTransaccion<F1, F2>(
    workspace_id: Uuid,
    cuentas: Vec<accounts::Cuenta>,
    categorias: Vec<accounting::Categoria>,
    transaccion_existente: Option<accounting::Transaccion>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    // Signal (no solo prop estática) para poder agregar una categoría
    // nueva sin cerrar el formulario y verla aparecer de inmediato en
    // el selector de abajo.
    let categorias = RwSignal::new(categorias);
    let es_edicion = transaccion_existente.is_some();
    let id_existente = transaccion_existente.as_ref().map(|t| t.id);

    let tipo = RwSignal::new(
        transaccion_existente
            .as_ref()
            .map(|t| t.tipo.clone())
            .unwrap_or_else(|| "expense".to_string()),
    );
    let monto = RwSignal::new(
        transaccion_existente
            .as_ref()
            .map(|t| t.amount.to_string())
            .unwrap_or_default(),
    );
    let fecha = RwSignal::new(
        transaccion_existente
            .as_ref()
            .map(|t| t.date.to_string())
            .unwrap_or_else(|| hoy().to_string()),
    );
    let cuenta_id = RwSignal::new(
        transaccion_existente
            .as_ref()
            .map(|t| t.account_id.to_string())
            .or_else(|| cuentas.first().map(|c| c.id.to_string()))
            .unwrap_or_default(),
    );
    let categoria_id = RwSignal::new(
        transaccion_existente
            .as_ref()
            .and_then(|t| t.category_id)
            .map(|id| id.to_string())
            .unwrap_or_default(),
    );
    let descripcion = RwSignal::new(
        transaccion_existente
            .as_ref()
            .and_then(|t| t.description.clone())
            .unwrap_or_default(),
    );
    let etiquetas_elegidas = RwSignal::new(Vec::<Uuid>::new());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let etiquetas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        tags::listar_etiquetas(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let categorias_filtradas = move || {
        let tipo_actual = tipo.get();
        categorias
            .get()
            .into_iter()
            .filter(|c| c.tipo == tipo_actual)
            .collect::<Vec<_>>()
    };

    let nueva_categoria = RwSignal::new(String::new());
    let creando_categoria = RwSignal::new(false);
    let crear_categoria = move |_| {
        let nombre = nueva_categoria.get_untracked();
        if nombre.trim().is_empty() || creando_categoria.get_untracked() {
            return;
        }
        creando_categoria.set(true);
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let datos = accounting::CrearCategoriaDatos {
                    name: nombre.trim(),
                    tipo: &tipo.get_untracked(),
                };
                if let Ok(creada) = accounting::crear_categoria(workspace_id, &datos, &token).await
                {
                    categoria_id.set(creada.id.to_string());
                    categorias.update(|lista| lista.push(creada));
                    nueva_categoria.set(String::new());
                }
            }
            creando_categoria.set(false);
        });
    };

    let nueva_etiqueta = RwSignal::new(String::new());
    let creando_etiqueta = RwSignal::new(false);
    let crear_etiqueta = move |_| {
        let nombre = nueva_etiqueta.get_untracked();
        if nombre.trim().is_empty() || creando_etiqueta.get_untracked() {
            return;
        }
        creando_etiqueta.set(true);
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
                    nueva_etiqueta.set(String::new());
                }
            }
            creando_etiqueta.set(false);
        });
    };

    let guardar = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(account_id) = Uuid::parse_str(&cuenta_id.get_untracked()) else {
            error.set(Some("Elige una cuenta".to_string()));
            return;
        };
        let category_id = Uuid::parse_str(&categoria_id.get_untracked()).ok();
        let Ok(amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };
        let Ok(date) = fecha.get_untracked().parse() else {
            error.set(Some("La fecha no es válida".to_string()));
            return;
        };

        guardando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let descripcion_actual = descripcion.get_untracked();
            let datos = accounting::DatosTransaccion {
                tipo: &tipo.get_untracked(),
                amount,
                date,
                category_id,
                account_id,
                description: if descripcion_actual.trim().is_empty() {
                    None
                } else {
                    Some(descripcion_actual.trim())
                },
            };

            let resultado = if let Some(id) = id_existente {
                accounting::actualizar_transaccion(workspace_id, id, &datos, &token)
                    .await
                    .map(|_| ())
            } else {
                match accounting::crear_transaccion(workspace_id, &datos, &token).await {
                    Ok(creada) => {
                        // Las etiquetas solo se asocian al crear — editar una
                        // transacción no vuelve a tocar sus etiquetas (ver
                        // docs/frontend-ia.md, simplificación consciente de
                        // esta primera pasada del módulo).
                        for tag_id in etiquetas_elegidas.get_untracked() {
                            let _ = tags::agregar_etiqueta_a_transaccion(
                                workspace_id,
                                creada.id,
                                tag_id,
                                &token,
                            )
                            .await;
                        }
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
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
                    <select prop:value=move || tipo.get() on:change=move |ev| tipo.set(event_target_value(&ev))>
                        <option value="expense">"Gasto"</option>
                        <option value="income">"Ingreso"</option>
                    </select>
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
                    <label>"Fecha"</label>
                    <input
                        type="date"
                        prop:value=move || fecha.get()
                        on:input=move |ev| fecha.set(event_target_value(&ev))
                    />
                </div>
                <div class="field">
                    <label>"Cuenta"</label>
                    <select prop:value=move || cuenta_id.get() on:change=move |ev| cuenta_id.set(event_target_value(&ev))>
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
                    <div style="display:flex; gap:6px; margin-top:8px;">
                        <input
                            placeholder="Nueva categoría..."
                            style="flex:1;"
                            prop:value=move || nueva_categoria.get()
                            on:input=move |ev| nueva_categoria.set(event_target_value(&ev))
                        />
                        <button type="button" class="btn-ghost" on:click=crear_categoria>
                            "+"
                        </button>
                    </div>
                </div>
                <div class="field">
                    <label>"Descripción"</label>
                    <input
                        prop:value=move || descripcion.get()
                        on:input=move |ev| descripcion.set(event_target_value(&ev))
                    />
                </div>
            </div>

            <Show when=move || !es_edicion>
                <div class="field" style="margin-bottom:16px;">
                    <label>"Etiquetas"</label>
                    <div style="display:flex; gap:8px; flex-wrap:wrap; margin-top:4px;">
                        {move || {
                            etiquetas
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .map(|etiqueta| {
                                    let id = etiqueta.id;
                                    let activa = move || etiquetas_elegidas.get().contains(&id);
                                    let alternar = move |_| {
                                        etiquetas_elegidas.update(|actuales| {
                                            if let Some(pos) = actuales.iter().position(|x| *x == id) {
                                                actuales.remove(pos);
                                            } else {
                                                actuales.push(id);
                                            }
                                        });
                                    };
                                    view! {
                                        <button
                                            type="button"
                                            class=move || if activa() { "chip chip-toggle is-active" } else { "chip chip-toggle" }
                                            on:click=alternar
                                        >
                                            {etiqueta.name}
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>
                    <div style="display:flex; gap:6px; margin-top:10px;">
                        <input
                            placeholder="Nueva etiqueta..."
                            style="flex:1;"
                            prop:value=move || nueva_etiqueta.get()
                            on:input=move |ev| nueva_etiqueta.set(event_target_value(&ev))
                        />
                        <button type="button" class="btn-ghost" on:click=crear_etiqueta>
                            "+"
                        </button>
                    </div>
                </div>
            </Show>

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
                            "Crear movimiento"
                        }
                    }}
                </button>
            </div>
        </form>
    }
}
