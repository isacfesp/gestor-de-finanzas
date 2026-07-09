//! Pestaña "Metas": listado con progreso, alta/edición, aportes y
//! vista detalle con proyección de ahorro + historial de aportes
//! (backend `goals`).

use chrono::NaiveDate;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::util::{confirmar, hoy};
use crate::api::goals::{self, Meta};
use crate::auth::{token_vigente, use_auth};

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(Meta),
}

#[derive(Clone)]
enum Vista {
    Lista,
    Detalle(Meta),
}

/// Porcentaje de avance (0-100), misma fórmula que usa el backend en
/// `ProgresoMeta::percentage` — se calcula aquí para no pedir
/// `progreso_meta` por cada tarjeta del listado (esa llamada se reserva
/// para la vista detalle).
fn porcentaje(meta: &Meta) -> Decimal {
    if meta.target_amount.is_zero() {
        Decimal::ZERO
    } else {
        meta.current_amount * Decimal::from(100) / meta.target_amount
    }
}

#[component]
pub fn PestanaMetas(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let modo = RwSignal::new(ModoFormulario::Cerrado);
    let vista = RwSignal::new(Vista::Lista);
    let filtro_completadas = RwSignal::new(String::new());
    let error_lista = RwSignal::new(None::<String>);

    let metas = LocalResource::new(move || {
        let filtro = filtro_completadas.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let completadas = match filtro.as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
            goals::listar_metas(workspace_id, completadas, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let volver = move || {
        vista.set(Vista::Lista);
        metas.refetch();
    };

    view! {
        <section class="panel">
            {move || match vista.get() {
                Vista::Detalle(meta) => view! {
                    <DetalleMeta workspace_id=workspace_id meta=meta on_volver=volver/>
                }
                .into_any(),
                Vista::Lista => view! {
                    <div>
                        <div class="panel-head">
                            <h2>"Metas de ahorro"</h2>
                            <div style="display:flex; gap:10px; align-items:center;">
                                <select
                                    style="max-width:160px;"
                                    prop:value=move || filtro_completadas.get()
                                    on:change=move |ev| filtro_completadas.set(event_target_value(&ev))
                                >
                                    <option value="">"Todas"</option>
                                    <option value="false">"En curso"</option>
                                    <option value="true">"Completadas"</option>
                                </select>
                                <button
                                    class="btn btn-primary"
                                    style="padding:8px 15px; font-size:12.5px;"
                                    on:click=move |_| modo.set(ModoFormulario::Crear)
                                >
                                    "+ Nueva meta"
                                </button>
                            </div>
                        </div>

                        <Show when=move || error_lista.get().is_some()>
                            <p class="banner banner-error" style="margin-bottom:14px;">
                                {move || error_lista.get().unwrap_or_default()}
                            </p>
                        </Show>

                        {move || match modo.get() {
                            ModoFormulario::Cerrado => ().into_any(),
                            ModoFormulario::Crear => view! {
                                <FormularioMeta
                                    workspace_id=workspace_id
                                    meta_existente=None
                                    on_guardado=move || { modo.set(ModoFormulario::Cerrado); metas.refetch(); }
                                    on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                                />
                            }
                            .into_any(),
                            ModoFormulario::Editar(m) => view! {
                                <FormularioMeta
                                    workspace_id=workspace_id
                                    meta_existente=Some(m)
                                    on_guardado=move || { modo.set(ModoFormulario::Cerrado); metas.refetch(); }
                                    on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                                />
                            }
                            .into_any(),
                        }}

                        {move || match metas.get() {
                            None => view! { <p class="text-soft">"Cargando metas..."</p> }.into_any(),
                            Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                            Some(Ok(lista)) if lista.is_empty() => {
                                view! { <p class="text-soft">"No hay metas registradas."</p> }.into_any()
                            }
                            Some(Ok(lista)) => view! {
                                <div>
                                    {lista
                                        .into_iter()
                                        .map(|m| {
                                            let para_detalle = m.clone();
                                            let para_editar = m.clone();
                                            let id = m.id;
                                            view! {
                                                <TarjetaMeta
                                                    meta=m
                                                    on_ver=move || vista.set(Vista::Detalle(para_detalle.clone()))
                                                    on_editar=move || modo.set(ModoFormulario::Editar(para_editar.clone()))
                                                    on_eliminar=move || {
                                                        if !confirmar("¿Eliminar esta meta?") {
                                                            return;
                                                        }
                                                        leptos::task::spawn_local(async move {
                                                            let Some(token) = token_vigente(auth).await else { return; };
                                                            match goals::eliminar_meta(workspace_id, id, &token).await {
                                                                Ok(()) => {
                                                                    error_lista.set(None);
                                                                    metas.refetch();
                                                                }
                                                                // Desviación deliberada del patrón "silencioso" de
                                                                // agenda (presupuestos/previstos ignoran el error de
                                                                // borrado): aquí sí hay un 409 con mensaje útil
                                                                // ("tiene aportes vinculados") que vale la pena mostrar.
                                                                Err(error_api) => error_lista.set(Some(error_api.to_string())),
                                                            }
                                                        });
                                                    }
                                                />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any(),
                        }}
                    </div>
                }
                .into_any(),
            }}
        </section>
    }
}

#[component]
fn TarjetaMeta<FV, FE, FB>(meta: Meta, on_ver: FV, on_editar: FE, on_eliminar: FB) -> impl IntoView
where
    FV: Fn() + 'static,
    FE: Fn() + 'static,
    FB: Fn() + 'static,
{
    let porcentaje_valor = porcentaje(&meta);
    // Decimal no tiene una conversión directa y confiable a u32 para el
    // ancho de la barra (en %); pasar por f64 vía el propio Display es
    // más simple que lidiar con los traits de conversión numérica.
    let porcentaje_f64: f64 = porcentaje_valor.to_string().parse().unwrap_or(0.0);
    let ancho = porcentaje_f64.clamp(0.0, 100.0).round() as u32;
    let clase_barra = if porcentaje_valor >= Decimal::from(100) {
        "budget-fill is-over"
    } else if porcentaje_valor >= Decimal::from(80) {
        "budget-fill is-warning"
    } else {
        "budget-fill"
    };

    view! {
        <div class="budget-card">
            <div class="budget-card-head">
                <h4>
                    {meta.name.clone()}
                    <Show when=move || meta.is_completed>
                        <span class="text-positive" style="margin-left:8px; font-size:11px; font-weight:700;">
                            "Completada"
                        </span>
                    </Show>
                </h4>
                <div style="display:flex; gap:10px; align-items:center;">
                    <button class="btn-ghost" style="padding:4px 8px; font-size:11px;" on:click=move |_| on_ver()>
                        "Ver detalle"
                    </button>
                    <button class="btn-ghost" style="padding:4px 8px; font-size:11px;" on:click=move |_| on_editar()>
                        "Editar"
                    </button>
                    <button
                        class="btn-ghost"
                        style="padding:4px 8px; font-size:11px; color:var(--negative);"
                        on:click=move |_| on_eliminar()
                    >
                        "Eliminar"
                    </button>
                </div>
            </div>
            <div class="budget-track">
                <div class=clase_barra style=format!("width:{ancho}%;")></div>
            </div>
            <p class="text-faint" style="margin:6px 0 0; font-size:12px;">
                {format!("{:.2} / {:.2} · Vence: {}", meta.current_amount, meta.target_amount, meta.deadline)}
            </p>
        </div>
    }
}

#[component]
fn FormularioMeta<F1, F2>(
    workspace_id: Uuid,
    meta_existente: Option<Meta>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let es_edicion = meta_existente.is_some();
    let id_existente = meta_existente.as_ref().map(|m| m.id);

    let nombre = RwSignal::new(
        meta_existente
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default(),
    );
    let monto_objetivo = RwSignal::new(
        meta_existente
            .as_ref()
            .map(|m| m.target_amount.to_string())
            .unwrap_or_default(),
    );
    let fecha_limite = RwSignal::new(
        meta_existente
            .as_ref()
            .map(|m| m.deadline.to_string())
            .unwrap_or_else(|| hoy().to_string()),
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
        let Ok(target_amount) = monto_objetivo.get_untracked().parse() else {
            error.set(Some("El monto objetivo no es un número válido".to_string()));
            return;
        };
        let Ok(deadline) = fecha_limite.get_untracked().parse::<NaiveDate>() else {
            error.set(Some("La fecha límite no es válida".to_string()));
            return;
        };

        guardando.set(true);
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let nombre_actual = nombre.get_untracked();
            let datos = goals::DatosMeta {
                name: &nombre_actual,
                target_amount,
                deadline,
            };

            let resultado = if let Some(id) = id_existente {
                goals::actualizar_meta(workspace_id, id, &datos, &token)
                    .await
                    .map(|_| ())
            } else {
                goals::crear_meta(workspace_id, &datos, &token)
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
                    <label>"Monto objetivo"</label>
                    <input
                        placeholder="0.00"
                        prop:value=move || monto_objetivo.get()
                        on:input=move |ev| monto_objetivo.set(event_target_value(&ev))
                    />
                </div>
                <div class="field">
                    <label>"Fecha límite"</label>
                    <input
                        type="date"
                        prop:value=move || fecha_limite.get()
                        on:input=move |ev| fecha_limite.set(event_target_value(&ev))
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
                            "Crear meta"
                        }
                    }}
                </button>
            </div>
        </form>
    }
}

#[component]
fn DetalleMeta<FV>(workspace_id: Uuid, meta: Meta, on_volver: FV) -> impl IntoView
where
    FV: Fn() + 'static + Copy,
{
    let auth = use_auth();
    let meta_actual = RwSignal::new(meta);
    let periodo = RwSignal::new("monthly".to_string());
    let mostrar_aporte = RwSignal::new(false);

    let progreso = LocalResource::new(move || {
        let id = meta_actual.get().id;
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            goals::progreso_meta(workspace_id, id, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let proyeccion = LocalResource::new(move || {
        let id = meta_actual.get().id;
        let periodo_actual = periodo.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            goals::proyeccion_meta(workspace_id, id, &periodo_actual, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let aportes = LocalResource::new(move || {
        let id = meta_actual.get().id;
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            goals::listar_aportes(workspace_id, id, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    view! {
        <div>
            <div class="panel-head">
                <button class="btn-ghost" on:click=move |_| on_volver()>"← Volver"</button>
                <h2>{move || meta_actual.get().name}</h2>
            </div>

            {move || match progreso.get() {
                None => view! { <p class="text-soft">"Cargando progreso..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(p)) => {
                    let porcentaje_f64: f64 = p.percentage.to_string().parse().unwrap_or(0.0);
                    let ancho = porcentaje_f64.clamp(0.0, 100.0).round() as u32;
                    let clase_barra = if p.percentage >= Decimal::from(100) {
                        "budget-fill is-over"
                    } else if p.percentage >= Decimal::from(80) {
                        "budget-fill is-warning"
                    } else {
                        "budget-fill"
                    };
                    view! {
                        <div class="budget-card">
                            <div class="budget-card-head">
                                <h4>"Progreso"</h4>
                                <span class="text-soft" style="font-size:12px;">
                                    {format!("{:.2} / {:.2}", p.current_amount, p.target_amount)}
                                </span>
                            </div>
                            <div class="budget-track">
                                <div class=clase_barra style=format!("width:{ancho}%;")></div>
                            </div>
                            <p class="text-faint" style="margin:6px 0 0; font-size:12px;">
                                {format!("{:.0}% completado · restan {:.2}", p.percentage, p.remaining_amount)}
                            </p>
                        </div>
                    }
                    .into_any()
                }
            }}

            <div class="panel" style="margin-top:16px;">
                <div class="panel-head">
                    <h3>"Proyección de ahorro"</h3>
                    <select
                        prop:value=move || periodo.get()
                        on:change=move |ev| periodo.set(event_target_value(&ev))
                    >
                        <option value="monthly">"Mensual"</option>
                        <option value="weekly">"Semanal"</option>
                    </select>
                </div>
                {move || match proyeccion.get() {
                    None => view! { <p class="text-soft">"Calculando..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok(p)) => view! {
                        <p class="text-soft">
                            {format!(
                                "Necesitas aportar {:.2} cada {} durante {} periodos más para llegar a tiempo.",
                                p.aporte_necesario,
                                if p.periodo == "weekly" { "semana" } else { "mes" },
                                p.periodos_restantes,
                            )}
                        </p>
                    }
                    .into_any(),
                }}
            </div>

            <div class="panel" style="margin-top:16px;">
                <div class="panel-head">
                    <h3>"Aportes"</h3>
                    <button class="btn btn-primary" style="padding:8px 15px; font-size:12.5px;" on:click=move |_| mostrar_aporte.set(true)>
                        "+ Registrar aporte"
                    </button>
                </div>

                <Show when=move || mostrar_aporte.get()>
                    <FormularioAporte
                        workspace_id=workspace_id
                        meta_id=meta_actual.get().id
                        on_guardado=move |meta_actualizada: Meta| {
                            meta_actual.set(meta_actualizada);
                            mostrar_aporte.set(false);
                            progreso.refetch();
                            proyeccion.refetch();
                            aportes.refetch();
                        }
                        on_cancelar=move || mostrar_aporte.set(false)
                    />
                </Show>

                {move || match aportes.get() {
                    None => view! { <p class="text-soft">"Cargando aportes..."</p> }.into_any(),
                    Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                    Some(Ok(lista)) if lista.is_empty() => {
                        view! { <p class="text-soft">"Todavía no se ha registrado ningún aporte."</p> }.into_any()
                    }
                    Some(Ok(lista)) => view! {
                        <div class="table-scroll">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th>"Fecha"</th>
                                        <th>"Quién"</th>
                                        <th>"Tipo"</th>
                                        <th>"Monto"</th>
                                        <th>"Descripción"</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {lista
                                        .into_iter()
                                        .map(|a| {
                                            let signo = if a.tipo == "income" { "+" } else { "-" };
                                            let color = if a.tipo == "income" { "var(--positive)" } else { "var(--negative)" };
                                            view! {
                                                <tr>
                                                    <td>{a.date.to_string()}</td>
                                                    <td>{a.created_by_name.clone()}</td>
                                                    <td>{if a.tipo == "income" { "Ingreso" } else { "Retiro" }}</td>
                                                    <td class="num" style=format!("color:{color};")>{format!("{signo}{:.2}", a.amount)}</td>
                                                    <td>{a.description.clone().unwrap_or_else(|| "—".to_string())}</td>
                                                </tr>
                                            }
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
fn FormularioAporte<F1, F2>(
    workspace_id: Uuid,
    meta_id: Uuid,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn(Meta) + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let monto = RwSignal::new(String::new());
    let tipo = RwSignal::new("income".to_string());
    let fecha = RwSignal::new(hoy().to_string());
    let descripcion = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };
        let Ok(date) = fecha.get_untracked().parse::<NaiveDate>() else {
            error.set(Some("La fecha no es válida".to_string()));
            return;
        };

        guardando.set(true);
        let tipo_actual = tipo.get_untracked();
        let descripcion_actual = descripcion.get_untracked();
        leptos::task::spawn_local(async move {
            let Some(token) = token_vigente(auth).await else {
                error.set(Some("Sesión vencida".to_string()));
                guardando.set(false);
                return;
            };

            let datos = goals::DatosAporte {
                amount,
                tipo: Some(tipo_actual.as_str()),
                date,
                description: if descripcion_actual.trim().is_empty() {
                    None
                } else {
                    Some(descripcion_actual.trim())
                },
            };

            let resultado = goals::registrar_aporte(workspace_id, meta_id, &datos, &token).await;
            guardando.set(false);
            match resultado {
                Ok(meta_actualizada) => on_guardado(meta_actualizada),
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
                        <option value="income">"Sumar al ahorro"</option>
                        <option value="expense">"Retirar del ahorro"</option>
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
                    <label>"Descripción (opcional)"</label>
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
                    {move || if guardando.get() { "Guardando..." } else { "Registrar aporte" }}
                </button>
            </div>
        </form>
    }
}
