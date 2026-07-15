//! Pestaña "Transacciones": un solo formulario para las 4 operaciones
//! del día a día (Ingreso, Gasto, Ahorro, Transferencia — ver
//! `docs/frontend-ia.md`) y la tabla combinada de movimientos. Las
//! categorías y etiquetas que ofrece el formulario se gestionan en la
//! pestaña "Categorías y Etiquetas" (`categorias_tab.rs`), no aquí.
//!
//! "Ahorro" no existe como tipo en el backend: es un ingreso (`type =
//! "income"`) contra una cuenta de tipo `savings`. Sacar dinero de tu
//! ahorro se hace con Transferencia (ahorro → efectivo/tarjeta), nunca
//! como un "egreso de ahorro" — así el saldo de ahorro solo baja cuando
//! el dinero de verdad se mueve a otra cuenta tuya.

use chrono::NaiveDate;
use leptos::ev::SubmitEvent;
use leptos::portal::Portal;
use leptos::prelude::*;
use uuid::Uuid;

use super::util::{confirmar, hoy};
use super::{FormularioCategoria, FormularioCuenta};
use crate::api::{accounting, accounts, tags};
use crate::auth::{token_vigente, use_auth};
use crate::components::hoja_inferior::HojaInferior;
use crate::components::icono_cuenta::IconoCuenta;
use crate::components::menu_flotante::{abrir_menu, estilo_posicion};

// ------------------------------ Operación ------------------------------

/// Las 4 operaciones que puede registrar el usuario. Cada una decide a
/// qué API se manda el formulario y qué tipos de cuenta puede elegir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operacion {
    Ingreso,
    Gasto,
    Ahorro,
    Transferencia,
}

impl Operacion {
    fn como_texto(self) -> &'static str {
        match self {
            Operacion::Ingreso => "Ingreso",
            Operacion::Gasto => "Gasto",
            Operacion::Ahorro => "Ahorro",
            Operacion::Transferencia => "Transferencia",
        }
    }

    fn desde_texto(texto: &str) -> Self {
        match texto {
            "Ingreso" => Operacion::Ingreso,
            "Ahorro" => Operacion::Ahorro,
            "Transferencia" => Operacion::Transferencia,
            _ => Operacion::Gasto,
        }
    }

    /// Tipo que se manda a `POST/PUT transacciones` — Transferencia no
    /// aplica (usa el endpoint de transferencias, no transacciones).
    fn tipo_backend(self) -> &'static str {
        match self {
            Operacion::Gasto => "expense",
            _ => "income",
        }
    }

    /// Tipos de cuenta seleccionables para esta operación. Una
    /// tarjeta de crédito no recibe "Ingreso" directo — solo sube de
    /// vuelta cuando se paga (Transferencia hacia ella); por eso
    /// `credit` queda fuera de Ingreso y solo aparece en Gasto (usarla)
    /// y Transferencia (pagarla o, si aplica, un retiro).
    fn tipos_cuenta(self) -> &'static [&'static str] {
        match self {
            Operacion::Ingreso => &["cash", "debit"],
            Operacion::Gasto => &["cash", "debit", "credit"],
            Operacion::Ahorro => &["savings"],
            Operacion::Transferencia => &["cash", "debit", "credit", "savings"],
        }
    }
}

/// Deriva la operación a partir del tipo de transacción y si su cuenta
/// es de ahorro (Ahorro no es un campo en la base: se infiere del tipo
/// de cuenta, ver comentario del módulo).
fn operacion_de(tipo: &str, es_ahorro: bool) -> Operacion {
    if tipo == "expense" {
        Operacion::Gasto
    } else if es_ahorro {
        Operacion::Ahorro
    } else {
        Operacion::Ingreso
    }
}

fn cuentas_para(op: Operacion, cuentas: &[accounts::Cuenta]) -> Vec<accounts::Cuenta> {
    cuentas
        .iter()
        .filter(|c| op.tipos_cuenta().contains(&c.tipo.as_str()))
        .cloned()
        .collect()
}

/// Las cuentas son personales: el formulario de operaciones (crear o
/// editar una transacción) solo puede operar sobre las del usuario
/// actual, aunque `cuentas` traiga también las ajenas (un admin/dev
/// las recibe para supervisión, ver `cuentas_tab.rs`).
fn cuentas_operables(cuentas: &[accounts::Cuenta], mi_id: Option<Uuid>) -> Vec<accounts::Cuenta> {
    cuentas
        .iter()
        .filter(|c| Some(c.owner_id) == mi_id)
        .cloned()
        .collect()
}

fn nombre_categoria(categorias: &[accounting::Categoria], id: Option<Uuid>) -> String {
    id.and_then(|id| categorias.iter().find(|c| c.id == id))
        .map(|c| c.name.clone())
        .unwrap_or_else(|| "Sin categoría".to_string())
}

fn categorias_de_tipo(
    categorias: &[accounting::Categoria],
    tipo: &str,
) -> Vec<accounting::Categoria> {
    categorias
        .iter()
        .filter(|c| c.tipo == tipo)
        .cloned()
        .collect()
}

// -------------------------- Tabla combinada -------------------------

/// Una fila ya lista para pintar: mezcla transacciones y transferencias,
/// que en el backend son dos tablas distintas pero en esta pantalla
/// conviven en un solo listado ordenado por fecha.
#[derive(Clone)]
struct FilaVista {
    fecha: NaiveDate,
    tipo_label: &'static str,
    descripcion: String,
    categoria: String,
    cuenta: String,
    tipo_cuenta: Option<String>,
    monto_texto: String,
    color: &'static str,
    quien: String,
    editable: Option<accounting::Transaccion>,
}

fn construir_filas(
    transacciones: &[accounting::TransaccionListado],
    transferencias: &[accounts::Transferencia],
    categorias: &[accounting::Categoria],
) -> Vec<FilaVista> {
    let mut filas = Vec::with_capacity(transacciones.len() + transferencias.len());

    for t in transacciones {
        let op = operacion_de(&t.tipo, t.account_tipo == "savings");
        let signo = if t.tipo == "income" { "+" } else { "-" };
        let color = if t.tipo == "income" {
            "var(--positive)"
        } else {
            "var(--negative)"
        };
        filas.push(FilaVista {
            fecha: t.date,
            tipo_label: op.como_texto(),
            descripcion: t.description.clone().unwrap_or_else(|| "—".to_string()),
            categoria: nombre_categoria(categorias, t.category_id),
            cuenta: t.account_name.clone(),
            tipo_cuenta: Some(t.account_tipo.clone()),
            monto_texto: format!("{signo}{:.2}", t.amount),
            color,
            quien: t.created_by_name.clone(),
            editable: Some(t.clone().into()),
        });
    }

    for tr in transferencias {
        filas.push(FilaVista {
            fecha: tr.date,
            tipo_label: "Transferencia",
            descripcion: tr.description.clone().unwrap_or_else(|| "—".to_string()),
            categoria: "—".to_string(),
            cuenta: format!("{} → {}", tr.from_account_name, tr.to_account_name),
            tipo_cuenta: None,
            monto_texto: format!("{:.2}", tr.amount),
            color: "var(--text)",
            // Transferencia no expone quién la creó en esta pasada
            // (fuera de alcance, ver plan de "cuentas personales").
            quien: "—".to_string(),
            editable: None,
        });
    }

    filas.sort_by_key(|f| std::cmp::Reverse(f.fecha));
    filas
}

#[derive(Clone)]
enum ModoFormulario {
    Cerrado,
    Crear,
    Editar(accounting::Transaccion),
}

#[component]
pub fn PestanaTransacciones(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let mi_id = move || auth.usuario().map(|u| u.id);
    let modo = RwSignal::new(ModoFormulario::Cerrado);

    let filtro_operacion = RwSignal::new(String::new());
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
        let desde = filtro_desde.get();
        let hasta = filtro_hasta.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            let filtros = accounting::FiltrosTransacciones {
                tipo: None,
                category_id: None,
                desde: desde.parse().ok(),
                hasta: hasta.parse().ok(),
            };
            accounting::listar_transacciones(workspace_id, &filtros, &token)
                .await
                .map_err(|e| e.to_string())
        }
    });

    let transferencias = LocalResource::new(move || {
        let desde = filtro_desde.get();
        let hasta = filtro_hasta.get();
        async move {
            let Some(token) = token_vigente(auth).await else {
                return Err("Sesión vencida".to_string());
            };
            accounts::listar_transferencias(
                workspace_id,
                desde.parse().ok(),
                hasta.parse().ok(),
                &token,
            )
            .await
            .map_err(|e| e.to_string())
        }
    });

    let recargar_todo = move || {
        transacciones.refetch();
        transferencias.refetch();
    };

    view! {
        <section class="panel">
            <div class="panel-head">
                <h2>"Transacciones"</h2>
                <button
                    class="btn btn-primary"
                    style="padding:8px 15px; font-size:12.5px;"
                    on:click=move |_| modo.set(ModoFormulario::Crear)
                >
                    "+ Nueva operación"
                </button>
            </div>

            <div style="display:flex; gap:12px; margin-bottom:16px; flex-wrap:wrap;">
                <select
                    style="max-width:170px;"
                    prop:value=move || filtro_operacion.get()
                    on:change=move |ev| filtro_operacion.set(event_target_value(&ev))
                >
                    <option value="">"Todos"</option>
                    <option value="Ingreso">"Ingresos"</option>
                    <option value="Gasto">"Gastos"</option>
                    <option value="Ahorro">"Ahorros"</option>
                    <option value="Transferencia">"Transferencias"</option>
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
                    <FormularioOperacion
                        workspace_id=workspace_id
                        cuentas=cuentas_operables(&cuentas.get().unwrap_or_default(), mi_id())
                        categorias=categorias.get().unwrap_or_default()
                        transaccion_existente=None
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); recargar_todo(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
                ModoFormulario::Editar(t) => view! {
                    <FormularioOperacion
                        workspace_id=workspace_id
                        cuentas=cuentas_operables(&cuentas.get().unwrap_or_default(), mi_id())
                        categorias=categorias.get().unwrap_or_default()
                        transaccion_existente=Some(t)
                        on_guardado=move || { modo.set(ModoFormulario::Cerrado); recargar_todo(); }
                        on_cancelar=move || modo.set(ModoFormulario::Cerrado)
                    />
                }
                .into_any(),
            }}

            {move || {
                let categorias_ok = categorias.get().unwrap_or_default();
                match (transacciones.get(), transferencias.get()) {
                    (Some(Err(mensaje)), _) | (_, Some(Err(mensaje))) => {
                        view! { <p class="banner banner-error">{mensaje}</p> }.into_any()
                    }
                    (Some(Ok(ts)), Some(Ok(trs))) => {
                        let filtro = filtro_operacion.get();
                        let mut filas = construir_filas(&ts, &trs, &categorias_ok);
                        if !filtro.is_empty() {
                            filas.retain(|f| f.tipo_label == filtro);
                        }
                        if filas.is_empty() {
                            view! { <p class="text-soft">"No hay movimientos con estos filtros."</p> }.into_any()
                        } else {
                            view! {
                                <TablaMovimientos
                                    workspace_id=workspace_id
                                    filas=filas
                                    on_editar=move |t| modo.set(ModoFormulario::Editar(t))
                                    on_borrada=move || recargar_todo()
                                />
                            }
                            .into_any()
                        }
                    }
                    _ => view! { <p class="text-soft">"Cargando movimientos..."</p> }.into_any(),
                }
            }}
        </section>
    }
}

#[component]
fn MenuFila<FE, FB>(on_editar: FE, on_borrar: FB) -> impl IntoView
where
    FE: Fn() + 'static + Copy + Send + Sync,
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
                        <button
                            type="button"
                            class="menu-item"
                            on:click=move |_| {
                                abierto.set(false);
                                on_editar();
                            }
                        >
                            "Editar"
                        </button>
                        <button
                            type="button"
                            class="menu-item is-danger"
                            on:click=move |_| {
                                abierto.set(false);
                                on_borrar();
                            }
                        >
                            "Borrar"
                        </button>
                    </div>
                </Portal>
            </Show>
        </div>
    }
}

#[component]
fn TablaMovimientos<FE, FB>(
    workspace_id: Uuid,
    filas: Vec<FilaVista>,
    on_editar: FE,
    on_borrada: FB,
) -> impl IntoView
where
    FE: Fn(accounting::Transaccion) + 'static + Copy + Send + Sync,
    FB: Fn() + 'static + Copy + Send + Sync,
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
        <div class="table-scroll">
            <table class="data-table">
                <thead>
                    <tr>
                        <th>"Fecha"</th>
                        <th>"Tipo"</th>
                        <th>"Descripción"</th>
                        <th>"Categoría"</th>
                        <th>"Cuenta"</th>
                        <th>"Quién"</th>
                        <th>"Monto"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {filas
                        .into_iter()
                        .map(|fila| {
                            let para_editar = fila.editable.clone();
                            view! {
                                <tr>
                                    <td>{fila.fecha.to_string()}</td>
                                    <td>{fila.tipo_label}</td>
                                    <td>{fila.descripcion}</td>
                                    <td>{fila.categoria}</td>
                                    <td>
                                        <span class="flex items-center gap-1.5">
                                            {fila.tipo_cuenta.clone().map(|tipo| view! { <IconoCuenta tipo=tipo/> })}
                                            <span>{fila.cuenta}</span>
                                        </span>
                                    </td>
                                    <td>{fila.quien}</td>
                                    <td class="num" style=format!("color:{};", fila.color)>{fila.monto_texto}</td>
                                    <td>
                                        {para_editar
                                            .map(|t| {
                                                let id = t.id;
                                                // StoredValue: MenuFila necesita que sus
                                                // callbacks sean Copy (el menú se puede
                                                // abrir/cerrar muchas veces) y Transaccion
                                                // no lo es — se guarda en el arena reactivo
                                                // y se clona al vuelo desde un handle Copy.
                                                let guardada = StoredValue::new(t);
                                                view! {
                                                    <MenuFila
                                                        on_editar=move || on_editar(guardada.get_value())
                                                        on_borrar=move || borrar(id)
                                                    />
                                                }
                                                    .into_any()
                                            })
                                            .unwrap_or_else(|| view! { "" }.into_any())}
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
fn FormularioOperacion<F1, F2>(
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
    let es_edicion = transaccion_existente.is_some();
    let id_existente = transaccion_existente.as_ref().map(|t| t.id);

    // Signal (no solo prop estática): el select de categoría vive dentro
    // de un <Show>, que puede reconstruir sus hijos varias veces — un
    // Vec normal no es Copy y no se puede mover de vuelta a cada
    // reconstrucción; un RwSignal sí (mismo motivo que `cuentas` abajo).
    let categorias = RwSignal::new(categorias);

    let operacion_inicial = transaccion_existente
        .as_ref()
        .map(|t| {
            let es_ahorro = cuentas
                .iter()
                .any(|c| c.id == t.account_id && c.tipo == "savings");
            operacion_de(&t.tipo, es_ahorro)
        })
        .unwrap_or(Operacion::Gasto);
    let cuenta_id_inicial = transaccion_existente
        .as_ref()
        .map(|t| t.account_id.to_string())
        .or_else(|| {
            cuentas_para(operacion_inicial, &cuentas)
                .first()
                .map(|c| c.id.to_string())
        })
        .unwrap_or_default();
    let cuenta_destino_inicial = cuentas_para(Operacion::Transferencia, &cuentas)
        .get(1)
        .map(|c| c.id.to_string())
        .unwrap_or_default();

    // Signal (no solo prop estática) para poder leer la lista de cuentas
    // dentro de varios closures reactivos sin pelear con el borrow
    // checker (un Vec normal no es Copy; un RwSignal sí).
    let cuentas = RwSignal::new(cuentas);

    let operacion = RwSignal::new(operacion_inicial);
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
    let cuenta_id = RwSignal::new(cuenta_id_inicial);
    let cuenta_destino_id = RwSignal::new(cuenta_destino_inicial);
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

    // Creación rápida de categoría/cuenta sin salir de este formulario
    // — abren una `HojaInferior` en vez de obligar a ir a la pestaña
    // "Categorías y Etiquetas"/"Cuentas". Al crear, se agrega a la
    // lista local (ya se ve en el <select>) y se preselecciona.
    let mostrar_categoria_rapida = RwSignal::new(false);
    let mostrar_cuenta_rapida = RwSignal::new(false);

    let etiquetas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        tags::listar_etiquetas(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    // Al cambiar de operación, la cuenta elegida puede quedar fuera del
    // nuevo filtro (ej. tenías "Efectivo" y pasas a Ahorro) — se reasigna
    // a la primera cuenta válida para no mandar una cuenta incompatible.
    // La categoría se limpia por la misma razón: Ingreso/Ahorro y Gasto
    // filtran categorías de distinto tipo (el backend valida que
    // coincidan), y una ya elegida del tipo viejo quedaría inválida sin
    // que se note en el `<select>`.
    let cambiar_operacion = move |ev| {
        let nueva = Operacion::desde_texto(&event_target_value(&ev));
        operacion.set(nueva);
        let disponibles = cuentas_para(nueva, &cuentas.get_untracked());
        cuenta_id.set(
            disponibles
                .first()
                .map(|c| c.id.to_string())
                .unwrap_or_default(),
        );
        cuenta_destino_id.set(
            disponibles
                .get(1)
                .map(|c| c.id.to_string())
                .unwrap_or_default(),
        );
        categoria_id.set(String::new());
    };

    let guardar = move |ev: SubmitEvent| {
        ev.prevent_default();
        error.set(None);

        let Ok(amount) = monto.get_untracked().parse() else {
            error.set(Some("El monto no es un número válido".to_string()));
            return;
        };
        let Ok(date) = fecha.get_untracked().parse() else {
            error.set(Some("La fecha no es válida".to_string()));
            return;
        };

        if operacion.get_untracked() == Operacion::Transferencia {
            let Ok(from_id) = Uuid::parse_str(&cuenta_id.get_untracked()) else {
                error.set(Some("Elige la cuenta de origen".to_string()));
                return;
            };
            let Ok(to_id) = Uuid::parse_str(&cuenta_destino_id.get_untracked()) else {
                error.set(Some("Elige la cuenta de destino".to_string()));
                return;
            };
            if from_id == to_id {
                error.set(Some(
                    "La cuenta de origen y destino no pueden ser la misma".to_string(),
                ));
                return;
            }

            guardando.set(true);
            leptos::task::spawn_local(async move {
                let Some(token) = token_vigente(auth).await else {
                    error.set(Some("Sesión vencida".to_string()));
                    guardando.set(false);
                    return;
                };
                let descripcion_actual = descripcion.get_untracked();
                let resultado = accounts::crear_transferencia(
                    workspace_id,
                    &accounts::CrearTransferenciaDatos {
                        from_account_id: from_id,
                        to_account_id: to_id,
                        amount,
                        date,
                        description: if descripcion_actual.trim().is_empty() {
                            None
                        } else {
                            Some(descripcion_actual.trim())
                        },
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
            return;
        }

        let Ok(account_id) = Uuid::parse_str(&cuenta_id.get_untracked()) else {
            error.set(Some("Elige una cuenta".to_string()));
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

            let descripcion_actual = descripcion.get_untracked();
            let datos = accounting::DatosTransaccion {
                tipo: operacion.get_untracked().tipo_backend(),
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
                        // transacción no vuelve a tocar sus etiquetas
                        // (simplificación consciente de esta pasada).
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
                    <label>"Operación"</label>
                    <select prop:value=move || operacion.get().como_texto() on:change=cambiar_operacion>
                        <option value="Gasto">"Gasto"</option>
                        <option value="Ingreso">"Ingreso"</option>
                        <option value="Ahorro">"Ahorro"</option>
                        <Show when=move || !es_edicion>
                            <option value="Transferencia">"Transferencia"</option>
                        </Show>
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
                    <label>"Fecha"</label>
                    <input
                        type="date"
                        prop:value=move || fecha.get()
                        on:input=move |ev| fecha.set(event_target_value(&ev))
                    />
                </div>

                <Show when=move || operacion.get() == Operacion::Transferencia>
                    <div class="field">
                        <label>"De"</label>
                        <select prop:value=move || cuenta_id.get() on:change=move |ev| cuenta_id.set(event_target_value(&ev))>
                            {move || {
                                cuentas_para(Operacion::Transferencia, &cuentas.get())
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
                        <label>"A"</label>
                        <select
                            prop:value=move || cuenta_destino_id.get()
                            on:change=move |ev| cuenta_destino_id.set(event_target_value(&ev))
                        >
                            {move || {
                                cuentas_para(Operacion::Transferencia, &cuentas.get())
                                    .into_iter()
                                    .map(|c| {
                                        let id = c.id.to_string();
                                        view! { <option value=id.clone()>{c.name.clone()}</option> }
                                    })
                                    .collect_view()
                            }}
                        </select>
                    </div>
                </Show>

                <Show when=move || operacion.get() != Operacion::Transferencia>
                    <div class="field">
                        <label>"Cuenta"</label>
                        <div style="display:flex; gap:6px;">
                            <select
                                style="flex:1;"
                                prop:value=move || cuenta_id.get()
                                on:change=move |ev| cuenta_id.set(event_target_value(&ev))
                            >
                                {move || {
                                    cuentas_para(operacion.get(), &cuentas.get())
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
                                    categorias_de_tipo(&categorias.get(), operacion.get().tipo_backend())
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
                </Show>

                <HojaInferior abierto=mostrar_categoria_rapida>
                    <FormularioCategoria
                        workspace_id=workspace_id
                        tipo_inicial=operacion.get_untracked().tipo_backend()
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

                <div class="field">
                    <label>{move || if operacion.get() == Operacion::Transferencia { "Nota (opcional)" } else { "Descripción" }}</label>
                    <input
                        prop:value=move || descripcion.get()
                        on:input=move |ev| descripcion.set(event_target_value(&ev))
                    />
                </div>
            </div>

            <Show when=move || operacion.get() != Operacion::Transferencia && !es_edicion>
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
