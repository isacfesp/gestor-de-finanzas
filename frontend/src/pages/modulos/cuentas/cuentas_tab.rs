//! Pestaña "Cuentas": únicamente tarjetas de cuenta y su alta/edición
//! (activar/desactivar). Las transferencias viven en la pestaña
//! Transacciones — ver `transacciones_tab.rs`.

use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::api::accounts;
use crate::auth::{token_vigente, use_auth};
use crate::components::icono_cuenta::IconoCuenta;
use crate::workspace::use_workspace;

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(accounts::Cuenta),
}

#[component]
pub fn PestanaCuentas(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let workspace = use_workspace();
    let modo = RwSignal::new(ModoFormulario::Cerrado);
    let mi_id = move || auth.usuario().map(|u| u.id);

    let cuentas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        accounts::listar_cuentas(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    // Solo un admin/dev puede pedirlo (403 para member); se ignora el
    // error y se usa una lista vacía, ya que un member nunca llega a
    // ver la sección de supervisión donde se necesita.
    let miembros = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        accounts::listar_miembros(workspace_id, &token)
            .await
            .unwrap_or_default()
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
                Some(Ok(lista)) => {
                    let (propias, ajenas): (Vec<_>, Vec<_>) = lista
                        .into_iter()
                        .partition(|c| Some(c.owner_id) == mi_id());

                    view! {
                        {if propias.is_empty() {
                            view! { <p class="text-soft">"Todavía no hay cuentas."</p> }.into_any()
                        } else {
                            view! {
                                <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(240px,1fr)); gap:16px;">
                                    {propias
                                        .into_iter()
                                        .map(|cuenta| {
                                            let para_editar = cuenta.clone();
                                            view! {
                                                <TarjetaCuenta
                                                    cuenta=cuenta
                                                    editable=true
                                                    on_editar=move || modo.set(ModoFormulario::Editar(para_editar.clone()))
                                                />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }}

                        {
                            let hay_ajenas = !ajenas.is_empty();
                            let miembros_ok = miembros.get().unwrap_or_default();
                            view! {
                                <Show when=move || workspace.puede_supervisar() && hay_ajenas>
                                    <h3 class="text-soft" style="margin:24px 0 12px; font-size:13px; font-weight:600;">
                                        "Cuentas de otros miembros (supervisión)"
                                    </h3>
                                    <div style="display:grid; grid-template-columns:repeat(auto-fill, minmax(240px,1fr)); gap:16px;">
                                        {ajenas
                                            .clone()
                                            .into_iter()
                                            .map(|cuenta| {
                                                let propietario = nombre_propietario(&miembros_ok, cuenta.owner_id);
                                                view! {
                                                    <TarjetaCuenta cuenta=cuenta editable=false propietario=propietario on_editar=|| {}/>
                                                }
                                            })
                                            .collect_view()}
                                    </div>
                                </Show>
                            }
                        }
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

fn nombre_propietario(miembros: &[accounts::MiembroBasico], owner_id: Uuid) -> Option<String> {
    miembros
        .iter()
        .find(|m| m.user_id == owner_id)
        .map(|m| m.name.clone())
}

#[component]
fn TarjetaCuenta<F>(
    cuenta: accounts::Cuenta,
    editable: bool,
    #[prop(optional_no_strip)] propietario: Option<String>,
    on_editar: F,
) -> impl IntoView
where
    F: Fn() + 'static,
{
    view! {
        <div class="panel" style="padding:20px;">
            <div style="display:flex; justify-content:space-between; align-items:flex-start;">
                <div class="flex items-start gap-2.5">
                    <span class="mt-0.5 flex h-8 w-8 flex-none items-center justify-center rounded-sm bg-hover text-muted">
                        <IconoCuenta tipo=cuenta.tipo.clone()/>
                    </span>
                    <div>
                        <h3 style="margin:0; font-size:16px; font-weight:700;">{cuenta.name.clone()}</h3>
                        <p class="text-soft" style="margin:3px 0 0; font-size:12px;">
                            {etiqueta_tipo(&cuenta.tipo)} " · " {cuenta.currency.clone()}
                        </p>
                    </div>
                </div>
                {if editable {
                    view! {
                        <button class="btn-ghost" style="padding:4px 8px; font-size:11px;" on:click=move |_| on_editar()>
                            "Editar"
                        </button>
                    }
                    .into_any()
                } else {
                    let etiqueta = propietario
                        .clone()
                        .map(|nombre| format!("De {nombre}"))
                        .unwrap_or_else(|| "Solo lectura".to_string());
                    view! { <span class="text-faint" style="font-size:11px;">{etiqueta}</span> }.into_any()
                }}
            </div>
            {match (cuenta.tipo.as_str(), cuenta.credit_limit) {
                ("credit", Some(limite)) => {
                    // balance es deuda (negativo cuando hay algo usado):
                    // disponible = límite - usado.
                    let usado = if cuenta.balance < Decimal::ZERO { -cuenta.balance } else { Decimal::ZERO };
                    let disponible = limite - usado;
                    view! {
                        <div class="mono" style="margin-top:20px; font-size:24px; font-weight:800;">
                            {format!("{} {:.2}", cuenta.currency, disponible)}
                        </div>
                        <p class="text-faint" style="margin:4px 0 0; font-size:12px;">
                            "Disponible de " {format!("{} {:.2}", cuenta.currency, limite)}
                            " · usado " {format!("{:.2}", usado)}
                        </p>
                        <p class="text-faint" style="margin:2px 0 0; font-size:12px;">
                            {if cuenta.is_active { "Activa" } else { "Inactiva" }}
                        </p>
                    }
                    .into_any()
                }
                _ => view! {
                    <div class="mono" style="margin-top:20px; font-size:24px; font-weight:800;">
                        {format!("{} {:.2}", cuenta.currency, cuenta.balance)}
                    </div>
                    <p class="text-faint" style="margin:4px 0 0; font-size:12px;">
                        {if cuenta.is_active { "Activa" } else { "Inactiva" }}
                    </p>
                }
                .into_any(),
            }}
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
    let limite_credito = RwSignal::new(
        cuenta_existente
            .as_ref()
            .and_then(|c| c.credit_limit)
            .map(|l| l.to_string())
            .unwrap_or_default(),
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

        // El límite solo aplica a tarjetas de crédito; el backend lo
        // exige mayor a cero en ese caso e ignora el campo para el
        // resto de los tipos.
        let credit_limit = if tipo.get_untracked() == "credit" {
            match limite_credito.get_untracked().parse() {
                Ok(valor) if valor > rust_decimal::Decimal::ZERO => Some(valor),
                _ => {
                    error.set(Some(
                        "Las tarjetas de crédito requieren un límite mayor a cero".to_string(),
                    ));
                    return;
                }
            }
        } else {
            None
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
                        credit_limit,
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
                        credit_limit,
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
                <Show when=move || !es_edicion && tipo.get() != "credit">
                    <div class="field">
                        <label>"Saldo inicial"</label>
                        <input
                            placeholder="0.00"
                            inputmode="decimal"
                            prop:value=move || saldo_inicial.get()
                            on:input=move |ev| saldo_inicial.set(event_target_value(&ev))
                        />
                    </div>
                </Show>
                <Show when=move || tipo.get() == "credit">
                    <div class="field">
                        <label>"Límite de crédito"</label>
                        <input
                            placeholder="0.00"
                            inputmode="decimal"
                            prop:value=move || limite_credito.get()
                            on:input=move |ev| limite_credito.set(event_target_value(&ev))
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
