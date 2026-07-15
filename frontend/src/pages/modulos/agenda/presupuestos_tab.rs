//! Pestaña "Presupuestos": límite de gasto mensual por categoría, con
//! barra de progreso gastado/límite (backend `accounting::presupuestos`).
//! Crear y editar el límite usan el mismo endpoint (upsert por
//! categoría+mes) — ver `docs/modules/02_accounting.md`.

use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use super::util::{confirmar, mes_actual};
use crate::api::{accounting, agenda};
use crate::auth::{token_vigente, use_auth};
use crate::workspace::use_workspace;

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(agenda::EstadoPresupuesto),
}

#[component]
pub fn PestanaPresupuestos(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let workspace = use_workspace();
    let fecha_hoy = mes_actual().to_string();
    let mes = RwSignal::new(fecha_hoy[..7].to_string());
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

    let estado = LocalResource::new(move || {
        let mes_texto = mes.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let Ok(month) = format!("{mes_texto}-01").parse() else {
                return Ok(Vec::new());
            };
            agenda::estado_presupuestos(workspace_id, month, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let eliminar = move |id: Uuid| {
        if !confirmar("¿Eliminar este presupuesto?") {
            return;
        }
        leptos::task::spawn_local(async move {
            if let Some(token) = token_vigente(auth).await {
                let _ = agenda::eliminar_presupuesto(workspace_id, id, &token).await;
                estado.refetch();
            }
        });
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Presupuestos"</h2>
                <div style="display:flex; gap:10px; align-items:center;">
                    <input
                        type="month"
                        prop:value=move || mes.get()
                        on:input=move |ev| mes.set(event_target_value(&ev))
                    />
                    <button
                        class="btn btn-primary"
                        style="padding:8px 15px; font-size:12.5px;"
                        on:click=move |_| modo.set(ModoFormulario::Crear)
                    >
                        "+ Nuevo presupuesto"
                    </button>
                </div>
            </div>

            {move || {
                let mes_texto = mes.get();
                let Ok(month) = format!("{mes_texto}-01").parse() else {
                    return ().into_any();
                };
                // Solo las categorías que YO ya presupuesté: cada usuario
                // tiene su propio límite, así que la de otro miembro no
                // debe bloquear la mía para la misma categoría.
                let ya_presupuestadas: Vec<Uuid> = estado
                    .get()
                    .and_then(|r| r.ok())
                    .unwrap_or_default()
                    .iter()
                    .filter(|e| Some(e.owner_id) == mi_id())
                    .map(|e| e.category_id)
                    .collect();
                match modo.get() {
                    ModoFormulario::Cerrado => ().into_any(),
                    ModoFormulario::Crear => view! {
                        <FormularioPresupuesto
                            workspace_id=workspace_id
                            month=month
                            categorias={
                                categorias
                                    .get()
                                    .unwrap_or_default()
                                    .into_iter()
                                    .filter(|c| !ya_presupuestadas.contains(&c.id))
                                    .collect::<Vec<_>>()
                            }
                            presupuesto_existente=None
                            on_guardado=move || { modo.set(ModoFormulario::Cerrado); estado.refetch(); }
                            on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                        />
                    }
                    .into_any(),
                    ModoFormulario::Editar(p) => view! {
                        <FormularioPresupuesto
                            workspace_id=workspace_id
                            month=month
                            categorias=Vec::new()
                            presupuesto_existente=Some(p)
                            on_guardado=move || { modo.set(ModoFormulario::Cerrado); estado.refetch(); }
                            on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                        />
                    }
                    .into_any(),
                }
            }}

            {move || match estado.get() {
                None => view! { <p class="text-soft">"Cargando presupuestos..."</p> }.into_any(),
                Some(Err(mensaje)) => view! { <p class="banner banner-error">{mensaje}</p> }.into_any(),
                Some(Ok(lista)) if lista.is_empty() => {
                    view! { <p class="text-soft">"No hay presupuestos para este mes."</p> }.into_any()
                }
                Some(Ok(lista)) => {
                    let (propios, ajenos): (Vec<_>, Vec<_>) = lista
                        .into_iter()
                        .partition(|p| Some(p.owner_id) == mi_id());
                    let hay_ajenos = !ajenos.is_empty();
                    view! {
                        {if propios.is_empty() {
                            view! { <p class="text-soft">"No tienes presupuestos para este mes."</p> }.into_any()
                        } else {
                            view! {
                                <div>
                                    {propios
                                        .into_iter()
                                        .map(|p| {
                                            let para_editar = p.clone();
                                            let id = p.id;
                                            view! {
                                                <TarjetaPresupuesto
                                                    presupuesto=p
                                                    editable=true
                                                    on_editar=move || modo.set(ModoFormulario::Editar(para_editar.clone()))
                                                    on_eliminar=move || eliminar(id)
                                                />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }}
                        <Show when=move || workspace.puede_supervisar() && hay_ajenos>
                            <h3 class="text-soft" style="margin:24px 0 12px; font-size:13px; font-weight:600;">
                                "Presupuestos de otros miembros (supervisión)"
                            </h3>
                            <div>
                                {ajenos
                                    .clone()
                                    .into_iter()
                                    .map(|p| view! {
                                        <TarjetaPresupuesto presupuesto=p editable=false on_editar=|| {} on_eliminar=|| {}/>
                                    })
                                    .collect_view()}
                            </div>
                        </Show>
                    }
                    .into_any()
                }
            }}
        </section>
    }
}

#[component]
fn TarjetaPresupuesto<FE, FB>(
    presupuesto: agenda::EstadoPresupuesto,
    editable: bool,
    on_editar: FE,
    on_eliminar: FB,
) -> impl IntoView
where
    FE: Fn() + 'static,
    FB: Fn() + 'static,
{
    let porcentaje = presupuesto.percentage;
    // Decimal no tiene una conversión directa y confiable a u32 para el
    // ancho de la barra (en %); pasar por f64 vía el propio Display es
    // más simple que lidiar con los traits de conversión numérica.
    let porcentaje_f64: f64 = porcentaje.to_string().parse().unwrap_or(0.0);
    let ancho = porcentaje_f64.clamp(0.0, 100.0).round() as u32;
    let clase_barra = if porcentaje >= Decimal::from(100) {
        "budget-fill is-over"
    } else if porcentaje >= Decimal::from(80) {
        "budget-fill is-warning"
    } else {
        "budget-fill"
    };

    view! {
        <div class="budget-card">
            <div class="budget-card-head">
                <h4>{presupuesto.category_name.clone()}</h4>
                <div style="display:flex; gap:10px; align-items:center;">
                    <span class="text-soft" style="font-size:12px;">
                        {format!("{:.2} / {:.2}", presupuesto.spent, presupuesto.limit_amount)}
                    </span>
                    {if editable {
                        view! {
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
                        }
                        .into_any()
                    } else {
                        view! { <span class="text-faint" style="font-size:11px;">"Solo lectura"</span> }.into_any()
                    }}
                </div>
            </div>
            <div class="budget-track">
                <div class=clase_barra style=format!("width:{ancho}%;")></div>
            </div>
            <p class="text-faint" style="margin:6px 0 0; font-size:12px;">
                {format!("{:.0}% usado", porcentaje)}
            </p>
        </div>
    }
}

#[component]
fn FormularioPresupuesto<F1, F2>(
    workspace_id: Uuid,
    month: chrono::NaiveDate,
    categorias: Vec<accounting::Categoria>,
    presupuesto_existente: Option<agenda::EstadoPresupuesto>,
    on_guardado: F1,
    on_cancelar: F2,
) -> impl IntoView
where
    F1: Fn() + 'static + Copy,
    F2: Fn() + 'static,
{
    let auth = use_auth();
    let es_edicion = presupuesto_existente.is_some();
    let nombre_categoria_fija = presupuesto_existente
        .as_ref()
        .map(|p| p.category_name.clone())
        .unwrap_or_default();
    let categoria_id_fija = presupuesto_existente.as_ref().map(|p| p.category_id);

    let categoria_id = RwSignal::new(
        categorias
            .first()
            .map(|c| c.id.to_string())
            .unwrap_or_default(),
    );
    let limite = RwSignal::new(
        presupuesto_existente
            .as_ref()
            .map(|p| p.limit_amount.to_string())
            .unwrap_or_default(),
    );
    let error = RwSignal::new(None::<String>);
    let guardando = RwSignal::new(false);

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(limit_amount) = limite.get_untracked().parse() else {
            error.set(Some("El límite no es un número válido".to_string()));
            return;
        };
        let category_id = if let Some(id) = categoria_id_fija {
            id
        } else {
            match Uuid::parse_str(&categoria_id.get_untracked()) {
                Ok(id) => id,
                Err(_) => {
                    error.set(Some("Elige una categoría".to_string()));
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
            let resultado = agenda::crear_presupuesto(
                workspace_id,
                &agenda::DatosPresupuesto {
                    category_id,
                    month,
                    limit_amount,
                },
                &token,
            )
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
                    <label>"Categoría"</label>
                    {if es_edicion {
                        view! { <input prop:value=nombre_categoria_fija.clone() disabled=true/> }.into_any()
                    } else {
                        view! {
                            <select
                                prop:value=move || categoria_id.get()
                                on:change=move |ev| categoria_id.set(event_target_value(&ev))
                            >
                                {categorias
                                    .iter()
                                    .map(|c| {
                                        let id = c.id.to_string();
                                        view! { <option value=id.clone()>{c.name.clone()}</option> }
                                    })
                                    .collect_view()}
                            </select>
                        }
                        .into_any()
                    }}
                </div>
                <div class="field">
                    <label>"Límite mensual"</label>
                    <input
                        placeholder="0.00"
                        inputmode="decimal"
                        prop:value=move || limite.get()
                        on:input=move |ev| limite.set(event_target_value(&ev))
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
                            "Crear presupuesto"
                        }
                    }}
                </button>
            </div>
        </form>
    }
}
