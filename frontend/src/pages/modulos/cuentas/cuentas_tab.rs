//! Pestaña "Cuentas": tarjetas de cuenta, alta/edición y transferencias.

use leptos::prelude::*;
use uuid::Uuid;

use super::util::hoy;
use crate::api::accounts;
use crate::auth::{token_vigente, use_auth};

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(accounts::Cuenta),
}

#[component]
pub fn PestanaCuentas(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let modo = RwSignal::new(ModoFormulario::Cerrado);

    let cuentas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        accounts::listar_cuentas(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Cuentas"</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| modo.set(ModoFormulario::Crear)
                >
                    "+ Nueva cuenta"
                </button>
            </div>

            {move || match modo.get() {
                ModoFormulario::Cerrado => ().into_any(),
                ModoFormulario::Crear => view! {
                    <FormularioCuenta
                        workspace_id=workspace_id
                        cuenta_existente=None
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); cuentas.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
                ModoFormulario::Editar(cuenta) => view! {
                    <FormularioCuenta
                        workspace_id=workspace_id
                        cuenta_existente=Some(cuenta)
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); cuentas.refetch(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
            }}

            {move || match cuentas.get() {
                None => view! { <p class="text-soft">"Cargando cuentas..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"Todavía no hay cuentas."</p> }.into_any()
                }
                Some(Ok(lista)) => view! {
                    <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(240px,1fr)); gap:16px;">
                        {lista
                            .into_iter()
                            .map(|cuenta| {
                                let para_editar = cuenta.clone();
                                view! {
                                    <TarjetaCuenta
                                        cuenta=cuenta
                                        on_editar=move || modo.set(ModoFormulario::Editar(para_editar.clone()))
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any(),
            }}
        </section>

        {move || match cuentas.get() {
            Some(Ok(lista)) => view! { <SeccionTransferencias workspace_id=workspace_id cuentas=lista/> }.into_any(),
            _ => ().into_any(),
        }}
    }
}

#[component]
fn TarjetaCuenta<F>(cuenta: accounts::Cuenta, on_editar: F) -> impl IntoView
where
    F: Fn() + 'static,
{
    view! {
        <div class="panel" style="padding:20px;">
            <div style="display:flex; justify-content:space-between; align-items:flex-start;">
                <div>
                    <h3 style="margin:0; font-size:16px; font-weight:700;">{cuenta.name.clone()}</h3>
                    <p class="text-soft" style="margin:3px 0 0; font-size:12px;">
                        {etiqueta_tipo(&cuenta.tipo)} " · " {cuenta.currency.clone()}
                    </p>
                </div>
                <button class="btn-ghost" style="padding:4px 8px; font-size:11px;" on:click=move |_| on_editar()>
                    "Editar"
                </button>
            </div>
            <div class="mono" style="margin-top:20px; font-size:24px; font-weight:800;">
                {format!("{} {:.2}", cuenta.currency, cuenta.balance)}
            </div>
            <p class="text-faint" style="margin:4px 0 0; font-size:12px;">
                {if cuenta.is_active { "Activa" } else { "Inactiva" }}
            </p>
        </div>
    }
}

fn etiqueta_tipo(tipo: &str) -> &'static str {
    match tipo {
        "cash" => "Efectivo",
        "debit" => "Débito",
        "credit" => "Crédito",
        "savings" => "Ahorro",
        "investment" => "Inversión",
        _ => "Otro",
    }
}

#[component]
fn FormularioCuenta<F1, F2>(
    workspace_id: Uuid,
    cuenta_existente: Option<accounts::Cuenta>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let es_edicion = cuenta_existente.is_some();
    let id_existente = cuenta_existente.as_ref().map(|c| c.id);

    let nombre = RwSignal::new(
        cuenta_existente
            .as_ref()
            .map(|c| c.name.clone())
            .unwrap_or_default(),
    );
    let tipo = RwSignal::new(
        cuenta_existente
            .as_ref()
            .map(|c| c.tipo.clone())
            .unwrap_or_else(|| "cash".to_string()),
    );
    let moneda = RwSignal::new(
        cuenta_existente
            .as_ref()
            .map(|c| c.currency.clone())
            .unwrap_or_else(|| "MXN".to_string()),
    );
    let saldo_inicial = RwSignal::new(String::new());
    let activa = RwSignal::new(
        cuenta_existente
            .as_ref()
            .map(|c| c.is_active)
            .unwrap_or(true),
    );
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        if nombre.get_untracked().trim().is_empty() {
            error.set(Some("El nombre no puede estar vacío".to_string()));
            return;
        }

        let balance = if es_edicion || saldo_inicial.get_untracked().trim().is_empty() {
            None
        } else {
            match saldo_inicial.get_untracked().parse() {
                Ok(valor) => Some(valor),
                Err(_) => {
                    error.set(Some("El saldo inicial no es un número válido".to_string()));
                    return;
                }
            }
        };

        guardando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let resultado = if let Some(id) = id_existente {
                accounts::actualizar_cuenta(
                    workspace_id,
                    id,
                    &accounts::ActualizarCuentaDatos {
                        name: &nombre.get_untracked(),
                        tipo: &tipo.get_untracked(),
                        currency: &moneda.get_untracked(),
                        is_active: activa.get_untracked(),
                    },
                    &token,
                )
                .await
                .map(|_| ())
            } else {
                accounts::crear_cuenta(
                    workspace_id,
                    &accounts::CrearCuentaDatos {
                        name: &nombre.get_untracked(),
                        tipo: &tipo.get_untracked(),
                        balance,
                        currency: Some(&moneda.get_untracked()),
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
                    <label>"Tipo"</label>
                    <select prop:value=move || tipo.get() on:change=move |ev| tipo.set(event_target_value(&ev))>
                        <option value="cash">"Efectivo"</option>
                        <option value="debit">"Débito"</option>
                        <option value="credit">"Crédito"</option>
                        <option value="savings">"Ahorro"</option>
                        <option value="investment">"Inversión"</option>
                    </select>
                </div>
                <div class="field">
                    <label>"Moneda"</label>
                    <input prop:value=move || moneda.get() on:input=move |ev| moneda.set(event_target_value(&ev))/>
                </div>
                <Show when=move || !es_edicion>
                    <div class="field">
                        <label>"Saldo inicial"</label>
                        <input
                            placeholder="0.00"
                            prop:value=move || saldo_inicial.get()
                            on:input=move |ev| saldo_inicial.set(event_target_value(&ev))
                        />
                    </div>
                </Show>
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
                            "Crear cuenta"
                        }
                    }}
                </button>
            </div>
        </form>
    }
}

#[component]
fn SeccionTransferencias(workspace_id: Uuid, cuentas: Vec<accounts::Cuenta>) -> impl IntoView {
    let auth = use_auth();
    let mostrar = RwSignal::new(false);

    let historial = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        accounts::listar_transferencias(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    let cuentas_form = cuentas.clone();

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Transferencias"</h2>
                <button class="btn-ghost" on:click=move |_| mostrar.update(|v| *v = !*v)>
                    {move || if mostrar.get() { "Cerrar" } else { "+ Nueva transferencia" }}
                </button>
            </div>

            <Show when=move || mostrar.get()>
                <FormularioTransferencia
                    workspace_id=workspace_id
                    cuentas=cuentas_form.clone()
                    on_hecha=move || {
                        mostrar.set(false);
                        historial.refetch();
                    }
                />
            </Show>

            {move || match historial.get() {
                None => view! { <p class="text-soft">"Cargando..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"Sin transferencias todavía."</p> }.into_any()
                }
                Some(Ok(lista)) => {
                    let cuentas = cuentas.clone();
                    view! {
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>"Fecha"</th>
                                    <th>"De"</th>
                                    <th>"A"</th>
                                    <th>"Nota"</th>
                                    <th>"Monto"</th>
                                </tr>
                            </thead>
                            <tbody>
                                {lista
                                    .into_iter()
                                    .map(|t| {
                                        let de = nombre_cuenta(&cuentas, t.from_account_id);
                                        let a = nombre_cuenta(&cuentas, t.to_account_id);
                                        view! {
                                            <tr>
                                                <td>{t.date.to_string()}</td>
                                                <td>{de}</td>
                                                <td>{a}</td>
                                                <td class="text-soft">{t.description.clone().unwrap_or_else(|| "—".to_string())}</td>
                                                <td class="num">{format!("{:.2}", t.amount)}</td>
                                            </tr>
                                        }
                                    })
                                    .collect_view()}
                            </tbody>
                        </table>
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
        .unwrap_or_else(|| "Cuenta eliminada".to_string())
}

#[component]
fn FormularioTransferencia<F>(
    workspace_id: Uuid,
    cuentas: Vec<accounts::Cuenta>,
    on_hecha: F,
) -> impl IntoView
where
    F: Fn() + 'static + Copy,
{
    let auth = use_auth();
    let origen = RwSignal::new(
        cuentas
            .first()
            .map(|c| c.id.to_string())
            .unwrap_or_default(),
    );
    let destino = RwSignal::new(cuentas.get(1).map(|c| c.id.to_string()).unwrap_or_default());
    let monto = RwSignal::new(String::new());
    let descripcion = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let enviando = RwSignal::new(false);

    let opciones_origen = cuentas.clone();
    let opciones_destino = cuentas;

    let enviar = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(from_id) = Uuid::parse_str(&origen.get_untracked()) else {
            error.set(Some("Elige la cuenta de origen".to_string()));
            return;
        };
        let Ok(to_id) = Uuid::parse_str(&destino.get_untracked()) else {
            error.set(Some("Elige la cuenta de destino".to_string()));
            return;
        };
        if from_id == to_id {
            error.set(Some(
                "La cuenta de origen y destino no pueden ser la misma".to_string(),
            ));
            return;
        }
        let Ok(amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };

        enviando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                enviando.set(false);
                return;
            };

            let descripcion_actual = descripcion.get_untracked();
            let descripcion_opt = if descripcion_actual.trim().is_empty() {
                None
            } else {
                Some(descripcion_actual.trim())
            };

            let resultado = accounts::crear_transferencia(
                workspace_id,
                &accounts::CrearTransferenciaDatos {
                    from_account_id: from_id,
                    to_account_id: to_id,
                    amount,
                    date: hoy(),
                    description: descripcion_opt,
                },
                &token,
            )
            .await;

            enviando.set(false);
            match resultado {
                Ok(_) => on_hecha(),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=enviar>
            <div class="form-grid">
                <div class="field">
                    <label>"De"</label>
                    <select prop:value=move || origen.get() on:change=move |ev| origen.set(event_target_value(&ev))>
                        {opciones_origen
                            .iter()
                            .map(|c| {
                                let id = c.id.to_string();
                                view! { <option value=id.clone()>{c.name.clone()}</option> }
                            })
                            .collect_view()}
                    </select>
                </div>
                <div class="field">
                    <label>"A"</label>
                    <select prop:value=move || destino.get() on:change=move |ev| destino.set(event_target_value(&ev))>
                        {opciones_destino
                            .iter()
                            .map(|c| {
                                let id = c.id.to_string();
                                view! { <option value=id.clone()>{c.name.clone()}</option> }
                            })
                            .collect_view()}
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
                    <label>"Nota (opcional)"</label>
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
                <button type="submit" class="btn btn-primary" disabled=move || enviando.get()>
                    {move || if enviando.get() { "Enviando..." } else { "Transferir" }}
                </button>
            </div>
        </form>
    }
}
