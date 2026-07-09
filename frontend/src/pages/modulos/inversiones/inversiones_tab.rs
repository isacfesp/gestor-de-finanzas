//! Pestaña "Inversiones": listado con estado activa/vencida, alta, y
//! vista detalle con desglose de rendimiento (bruto/ISR/neto) e
//! historial de rendimientos reales acreditados (backend
//! `investments`).

use chrono::NaiveDate;
use leptos::ev::SubmitEvent;
use leptos::portal::Portal;
use leptos::prelude::*;
use uuid::Uuid;

use super::desglose::TarjetaDesglose;
use super::util::{confirmar, hoy};
use crate::api::investments::{self, Inversion};
use crate::auth::{token_vigente, use_auth};
use crate::components::menu_flotante::{abrir_menu, estilo_posicion};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EstadoInversion {
    Activa,
    Vencida,
    Inactiva,
}

/// "Vencida" es `end_date < hoy`, sin importar `is_active` (una
/// inversión real vence aunque nadie haya tocado la bandera manual);
/// "Inactiva" es solo para una inversión desactivada a mano antes de
/// su vencimiento — caso raro pero posible dado el esquema. Ver nota
/// en `api::investments::listar_inversiones`.
fn estado_de(inv: &Inversion, hoy_actual: NaiveDate) -> EstadoInversion {
    if inv.end_date < hoy_actual {
        EstadoInversion::Vencida
    } else if inv.is_active {
        EstadoInversion::Activa
    } else {
        EstadoInversion::Inactiva
    }
}

fn etiqueta_estado(estado: EstadoInversion) -> (&'static str, &'static str) {
    match estado {
        EstadoInversion::Activa => ("Activa", "var(--positive)"),
        EstadoInversion::Vencida => ("Vencida", "var(--muted)"),
        EstadoInversion::Inactiva => ("Inactiva", "var(--negative)"),
    }
}

#[derive(Clone)]
enum Vista {
    Lista,
    Detalle(Inversion),
}

#[component]
pub fn PestanaInversiones(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let mostrar_form = RwSignal::new(false);
    let vista = RwSignal::new(Vista::Lista);
    let filtro_estado = RwSignal::new(String::new());

    let inversiones = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        investments::listar_inversiones(workspace_id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    let volver = move || {
        vista.set(Vista::Lista);
        inversiones.refetch();
    };

    view! {
        <section class="panel">
            {move || match vista.get() {
                Vista::Detalle(inv) => view! {
                    <DetalleInversion workspace_id=workspace_id inversion=inv on_volver=volver/>
                }
                .into_any(),
                Vista::Lista => view! {
                    <div>
                        <div class="panel-head">
                            <h2>"Inversiones"</h2>
                            <button
                                class="btn btn-primary"
                                style="padding:8px 15px; font-size:12.5px;"
                                on:click=move |_| mostrar_form.set(true)
                            >
                                "+ Nueva inversión"
                            </button>
                        </div>

                        <div style="display:flex; gap:12px; margin-bottom:16px;">
                            <select
                                style="max-width:170px;"
                                prop:value=move || filtro_estado.get()
                                on:change=move |ev| filtro_estado.set(event_target_value(&ev))
                            >
                                <option value="">"Todas"</option>
                                <option value="activas">"Activas"</option>
                                <option value="vencidas">"Vencidas"</option>
                            </select>
                        </div>

                        <Show when=move || mostrar_form.get()>
                            <FormularioInversion
                                workspace_id=workspace_id
                                on_guardado=move || { mostrar_form.set(false); inversiones.refetch(); }
                                on_cancelar=move || mostrar_form.set(false)
                            />
                        </Show>

                        {move || match inversiones.get() {
                            None => view! { <p class="text-soft">"Cargando inversiones..."</p> }.into_any(),
                            Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                            Some(Ok(lista)) => {
                                let hoy_actual = hoy();
                                let filtro = filtro_estado.get();
                                let filtradas: Vec<_> = lista
                                    .into_iter()
                                    .filter(|inv| match filtro.as_str() {
                                        "activas" => estado_de(inv, hoy_actual) == EstadoInversion::Activa,
                                        "vencidas" => estado_de(inv, hoy_actual) == EstadoInversion::Vencida,
                                        _ => true,
                                    })
                                    .collect();
                                if filtradas.is_empty() {
                                    view! { <p class="text-soft">"No hay inversiones con este filtro."</p> }.into_any()
                                } else {
                                    view! {
                                        <TablaInversiones
                                            workspace_id=workspace_id
                                            lista=filtradas
                                            on_ver=move |inv| vista.set(Vista::Detalle(inv))
                                            on_cambio=move || inversiones.refetch()
                                        />
                                    }
                                    .into_any()
                                }
                            }
                        }}
                    </div>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn MenuInversion<FV, FB>(on_ver: FV, on_borrar: FB) -> impl IntoView
where
    FV: Fn() + 'static + Copy + Send + Sync,
    FB: Fn() + 'static + Copy + Send + Sync,
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
                        <button type="button" class="menu-item" on:click=move |_| { abierto.set(false); on_ver(); }>
                            "Ver detalle"
                        </button>
                        <button type="button" class="menu-item is-danger" on:click=move |_| { abierto.set(false); on_borrar(); }>
                            "Eliminar"
                        </button>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}

#[component]
fn TablaInversiones<FV, FA>(
    workspace_id: Uuid,
    lista: Vec<Inversion>,
    on_ver: FV,
    on_cambio: FA,
) -> impl IntoView
where
    FV: Fn(Inversion) + 'static + Copy + Send + Sync,
    FA: Fn() + 'static + Copy + Send + Sync,
{
    let auth = use_auth();

    view! {
        <div class="table-scroll">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Nombre"</th>
                        <th>"Capital"</th>
                        <th>"Tasa GAT"</th>
                        <th>"Tipo"</th>
                        <th>"Inicio"</th>
                        <th>"Vencimiento"</th>
                        <th>"Estado"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {lista
                        .into_iter()
                        .map(|inv| {
                            let guardada = StoredValue::new(inv.clone());
                            let id = inv.id;
                            let (etiqueta, color) = etiqueta_estado(estado_de(&inv, hoy()));
                            view! {
                                <tr>
                                    <td>{inv.name.clone()}</td>
                                    <td class="num">{format!("{:.2}", inv.principal)}</td>
                                    <td class="num">{format!("{:.2}%", inv.gat_annual_rate)}</td>
                                    <td>{investments::etiqueta_tipo_interes(&inv.interest_type)}</td>
                                    <td>{inv.start_date.to_string()}</td>
                                    <td>{inv.end_date.to_string()}</td>
                                    <td style=format!("color:{color};")>{etiqueta}</td>
                                    <td>
                                        <MenuInversion
                                            on_ver=move || on_ver(guardada.get_value())
                                            on_borrar=move || {
                                                if !confirmar("¿Eliminar esta inversión? Se borrará también su historial de rendimientos.") {
                                                    return;
                                                }
                                                leptos::task::spawn_local(async move {
                                                    if let Some(token) = token_vigente(auth).await {
                                                        let _ = investments::eliminar_inversion(workspace_id, id, &token).await;
                                                        on_cambio();
                                                    }
                                                });
                                            }
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
fn FormularioInversion<F1, F2>(
    workspace_id: Uuid,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let nombre = RwSignal::new(String::new());
    let principal = RwSignal::new(String::new());
    let tasa = RwSignal::new(String::new());
    let tipo_interes = RwSignal::new("simple".to_string());
    let fecha_inicio = RwSignal::new(hoy().to_string());
    let plazo = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        if nombre.get_untracked().trim().is_empty() {
            error.set(Some("El nombre no puede estar vacío".to_string()));
            return;
        }
        let Ok(principal_val) = principal.get_untracked().parse() else {
            error.set(Some("El capital no es un número válido".to_string()));
            return;
        };
        let Ok(tasa_val) = tasa.get_untracked().parse() else {
            error.set(Some("La tasa no es un número válido".to_string()));
            return;
        };
        let Ok(start_date) = fecha_inicio.get_untracked().parse::<NaiveDate>() else {
            error.set(Some("La fecha de inicio no es válida".to_string()));
            return;
        };
        let Ok(term_days) = plazo.get_untracked().parse() else {
            error.set(Some("El plazo no es un número válido".to_string()));
            return;
        };

        guardando.set(true);
        let nombre_actual = nombre.get_untracked();
        let tipo_actual = tipo_interes.get_untracked();
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let datos = investments::DatosInversion {
                name: &nombre_actual,
                principal: principal_val,
                gat_annual_rate: tasa_val,
                interest_type: &tipo_actual,
                start_date,
                term_days,
            };

            let resultado = investments::crear_inversion(workspace_id, &datos, &token).await;
            guardando.set(false);
            match resultado {
                Ok(_) => on_guardado(),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Nombre"</label>
                    <input prop:value=move || nombre.get() on:input=move |ev| nombre.set(event_target_value(&ev)) required/>
                </div>
                <div class="field">
                    <label>"Capital"</label>
                    <input placeholder="0.00" prop:value=move || principal.get() on:input=move |ev| principal.set(event_target_value(&ev))/>
                </div>
                <div class="field">
                    <label>"Tasa GAT anual (%)"</label>
                    <input placeholder="0.00" prop:value=move || tasa.get() on:input=move |ev| tasa.set(event_target_value(&ev))/>
                </div>
                <div class="field">
                    <label>"Tipo de interés"</label>
                    <select prop:value=move || tipo_interes.get() on:change=move |ev| tipo_interes.set(event_target_value(&ev))>
                        {investments::TIPOS_INTERES
                            .iter()
                            .map(|(valor, etiqueta)| view! { <option value=*valor>{*etiqueta}</option> })
                            .collect_view()}
                    </select>
                </div>
                <div class="field">
                    <label>"Fecha de inicio"</label>
                    <input type="date" prop:value=move || fecha_inicio.get() on:input=move |ev| fecha_inicio.set(event_target_value(&ev))/>
                </div>
                <div class="field">
                    <label>"Plazo (días)"</label>
                    <input placeholder="0" prop:value=move || plazo.get() on:input=move |ev| plazo.set(event_target_value(&ev))/>
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
                    {move || if guardando.get() { "Guardando..." } else { "Crear inversión" }}
                </button>
            </div>
        </form>
    }
}

#[component]
fn DetalleInversion<FV>(workspace_id: Uuid, inversion: Inversion, on_volver: FV) -> impl IntoView
where
    FV: Fn() + 'static + Copy,
{
    let auth = use_auth();
    let id = inversion.id;
    let mostrar_form_rendimiento = RwSignal::new(false);

    let proyeccion = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        investments::proyeccion_inversion(workspace_id, id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    let rendimientos = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Err("Sesión vencida".to_string());
        };
        investments::listar_rendimientos(workspace_id, id, &token)
            .await
            .map_err(|e| e.to_string())
    });

    view! {
        <div>
            <div class="panel-head">
                <button class="btn-ghost" on:click=move |_| on_volver()>"← Volver"</button>
                <h2>{inversion.name.clone()}</h2>
                <button
                    class="btn-ghost"
                    style="color:var(--negative);"
                    on:click=move |_| {
                        if !confirmar("¿Eliminar esta inversión? Se borrará también su historial de rendimientos.") {
                            return;
                        }
                        leptos::task::spawn_local(async move {
                            if let Some(token) = token_vigente(auth).await {
                                let _ = investments::eliminar_inversion(workspace_id, id, &token).await;
                                on_volver();
                            }
                        });
                    }
                >
                    "Eliminar inversión"
                </button>
            </div>

            {move || match proyeccion.get() {
                None => view! { <p class="text-soft">"Calculando rendimiento..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(d)) => view! { <TarjetaDesglose desglose=d/> }.into_any(),
            }}

            <div class="panel" style="margin-top:16px;">
                <div class="panel-head">
                    <h3>"Rendimientos acreditados"</h3>
                    <button
                        class="btn btn-primary"
                        style="padding:8px 15px; font-size:12.5px;"
                        on:click=move |_| mostrar_form_rendimiento.set(true)
                    >
                        "+ Registrar rendimiento"
                    </button>
                </div>

                <Show when=move || mostrar_form_rendimiento.get()>
                    <FormularioRendimiento
                        workspace_id=workspace_id
                        investment_id=id
                        on_guardado=move || { mostrar_form_rendimiento.set(false); rendimientos.refetch(); }
                        on_cancelar=move || mostrar_form_rendimiento.set(false)
                    />
                </Show>

                {move || match rendimientos.get() {
                    None => view! { <p class="text-soft">"Cargando..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok(lista)) if lista.is_empty() => {
                        view! { <p class="text-soft">"Todavía no se ha acreditado ningún rendimiento."</p> }.into_any()
                    }
                    Some(Ok(lista)) => view! {
                        <div class="table-scroll">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Fecha"</th>
                                        <th>"Monto"</th>
                                        <th>"Notas"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {lista
                                        .into_iter()
                                        .map(|r| view! {
                                            <tr>
                                                <td>{r.yield_date.to_string()}</td>
                                                <td class="num">{format!("{:.2}", r.yield_amount)}</td>
                                                <td>{r.notes.clone().unwrap_or_else(|| "—".to_string())}</td>
                                            </tr>
                                        })
                                        .collect_view()}
                                </tbody>
                            </table>
                        </div>
                    }
                    .into_any(),
                }}
            </div>
        </div>
    }
}

#[component]
fn FormularioRendimiento<F1, F2>(
    workspace_id: Uuid,
    investment_id: Uuid,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let monto = RwSignal::new(String::new());
    let fecha = RwSignal::new(hoy().to_string());
    let notas = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(yield_amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };
        let Ok(yield_date) = fecha.get_untracked().parse::<NaiveDate>() else {
            error.set(Some("La fecha no es válida".to_string()));
            return;
        };

        guardando.set(true);
        let notas_actuales = notas.get_untracked();
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let datos = investments::DatosRendimiento {
                yield_amount,
                yield_date,
                notes: if notas_actuales.trim().is_empty() {
                    None
                } else {
                    Some(notas_actuales.trim())
                },
            };

            let resultado =
                investments::registrar_rendimiento(workspace_id, investment_id, &datos, &token)
                    .await;
            guardando.set(false);
            match resultado {
                Ok(_) => on_guardado(),
                Err(error_api) => error.set(Some(error_api.to_string())),
            }
        });
    };

    view! {
        <form class="panel form-panel" on:submit=guardar>
            <div class="form-grid">
                <div class="field">
                    <label>"Monto"</label>
                    <input placeholder="0.00" prop:value=move || monto.get() on:input=move |ev| monto.set(event_target_value(&ev))/>
                </div>
                <div class="field">
                    <label>"Fecha"</label>
                    <input type="date" prop:value=move || fecha.get() on:input=move |ev| fecha.set(event_target_value(&ev))/>
                </div>
                <div class="field">
                    <label>"Notas (opcional)"</label>
                    <input prop:value=move || notas.get() on:input=move |ev| notas.set(event_target_value(&ev))/>
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
                    {move || if guardando.get() { "Guardando..." } else { "Registrar rendimiento" }}
                </button>
            </div>
        </form>
    }
}
