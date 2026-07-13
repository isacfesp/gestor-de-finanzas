//! Pestaña "Calendario": vista mensual que junta en un solo lugar los
//! eventos con fecha futura de Agenda (previstos, próximo cobro de
//! suscripciones) y de Inversiones (vencimiento de metas) — ver plan
//! de rediseño mobile-first. Es una vista de solo lectura: para editar
//! un evento, sus acciones (marcar pagado, editar, etc.) siguen
//! viviendo en la pestaña de ese dominio; aquí solo se listan y, al
//! tocar un día, se resumen en una hoja inferior.

use chrono::{Datelike, Days, Duration, NaiveDate};
use leptos::prelude::*;
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::api::{agenda, goals};
use crate::auth::{token_vigente, use_auth};
use crate::components::hoja_inferior::HojaInferior;

use super::util::mes_actual;

/// Un evento de un día del calendario, ya reducido a lo que hace falta
/// para pintarlo (no guarda el `Previsto`/`Suscripcion`/`Meta`
/// completo: esta vista es de solo lectura).
#[derive(Clone)]
struct EventoDia {
    etiqueta: String,
    monto: Option<Decimal>,
    tipo: TipoEvento,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TipoEvento {
    Previsto,
    Suscripcion,
    Meta,
}

impl TipoEvento {
    fn color(self) -> &'static str {
        match self {
            TipoEvento::Previsto => "var(--accent)",
            TipoEvento::Suscripcion => "var(--warning)",
            TipoEvento::Meta => "var(--positive)",
        }
    }

    fn etiqueta_tipo(self) -> &'static str {
        match self {
            TipoEvento::Previsto => "Previsto",
            TipoEvento::Suscripcion => "Suscripción",
            TipoEvento::Meta => "Meta",
        }
    }
}

/// El día 1 de cualquier año/mes calendario siempre es válido — mismo
/// principio que `util::mes_actual`.
fn primer_dia_del_mes(fecha: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(fecha.year(), fecha.month(), 1).expect("día 1 siempre es válido")
}

fn ultimo_dia_del_mes(primer_dia: NaiveDate) -> NaiveDate {
    let primer_dia_prox_mes = if primer_dia.month() == 12 {
        NaiveDate::from_ymd_opt(primer_dia.year() + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(primer_dia.year(), primer_dia.month() + 1, 1)
    }
    .expect("el mes siguiente siempre es válido");
    primer_dia_prox_mes - Duration::days(1)
}

fn mes_anterior(mes: NaiveDate) -> NaiveDate {
    primer_dia_del_mes(mes) - Duration::days(1)
}

fn mes_siguiente(mes: NaiveDate) -> NaiveDate {
    ultimo_dia_del_mes(mes) + Duration::days(1)
}

const MESES: [&str; 12] = [
    "enero",
    "febrero",
    "marzo",
    "abril",
    "mayo",
    "junio",
    "julio",
    "agosto",
    "septiembre",
    "octubre",
    "noviembre",
    "diciembre",
];

const DIAS_SEMANA: [&str; 7] = ["L", "M", "X", "J", "V", "S", "D"];

#[component]
pub fn PestanaCalendario(workspace_id: Uuid) -> impl IntoView {
    let auth = use_auth();
    let mes = RwSignal::new(mes_actual());
    let dia_seleccionado = RwSignal::new(None::<NaiveDate>);
    let hoja_abierta = RwSignal::new(false);

    let previstos = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        let filtros = agenda::FiltrosPrevistos {
            desde: Some(primer_dia_del_mes(mes.get())),
            hasta: Some(ultimo_dia_del_mes(mes.get())),
            pagado: None,
        };
        agenda::listar_previstos(workspace_id, &filtros, &token)
            .await
            .unwrap_or_default()
    });

    // Sin filtro de fecha en la API (`next_billing_date` es la próxima
    // ocurrencia, no una regla de recurrencia) — se pide una vez y se
    // filtra por mes en el cliente; si el próximo cobro cae fuera del
    // mes que se está viendo, simplemente no aparece.
    let suscripciones = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        agenda::listar_suscripciones(workspace_id, &token)
            .await
            .unwrap_or_default()
    });

    let metas = LocalResource::new(move || async move {
        let Some(token) = token_vigente(auth).await else {
            return Vec::new();
        };
        goals::listar_metas(workspace_id, None, &token)
            .await
            .unwrap_or_default()
    });

    // Día → eventos de ese día, ya filtrados al mes visible. Se
    // recalcula en cada render reactivo (los recursos son pocos
    // elementos, no hace falta memoizarlo aparte).
    let eventos_de = move |dia: NaiveDate| -> Vec<EventoDia> {
        let mut eventos = Vec::new();
        if let Some(lista) = previstos.get() {
            for p in lista.iter().filter(|p| p.due_date == dia) {
                eventos.push(EventoDia {
                    etiqueta: p
                        .description
                        .clone()
                        .unwrap_or_else(|| "Previsto".to_string()),
                    monto: Some(p.amount),
                    tipo: TipoEvento::Previsto,
                });
            }
        }
        if let Some(lista) = suscripciones.get() {
            for s in lista
                .iter()
                .filter(|s| s.is_active && s.next_billing_date == dia)
            {
                eventos.push(EventoDia {
                    etiqueta: s.name.clone(),
                    monto: Some(s.amount),
                    tipo: TipoEvento::Suscripcion,
                });
            }
        }
        if let Some(lista) = metas.get() {
            for m in lista
                .iter()
                .filter(|m| !m.is_completed && m.deadline == dia)
            {
                eventos.push(EventoDia {
                    etiqueta: m.name.clone(),
                    monto: Some(m.target_amount),
                    tipo: TipoEvento::Meta,
                });
            }
        }
        eventos
    };

    let abrir_dia = move |dia: NaiveDate| {
        dia_seleccionado.set(Some(dia));
        hoja_abierta.set(true);
    };

    view! {
        <div class="panel">
            <div class="mb-4 flex items-center justify-between">
                <button
                    type="button"
                    class="btn-ghost"
                    on:click=move |_| mes.set(mes_anterior(mes.get()))
                >
                    "←"
                </button>
                <h3 class="text-[15px] font-bold text-text">
                    {move || {
                        let m = mes.get();
                        format!("{} {}", MESES[m.month0() as usize], m.year())
                    }}
                </h3>
                <button
                    type="button"
                    class="btn-ghost"
                    on:click=move |_| mes.set(mes_siguiente(mes.get()))
                >
                    "→"
                </button>
            </div>

            <div class="grid grid-cols-7 gap-1 text-center text-[11px] font-semibold text-faint">
                {DIAS_SEMANA.iter().map(|d| view! { <span class="py-1">{*d}</span> }).collect_view()}
            </div>

            <div class="mt-1 grid grid-cols-7 gap-1">
                {move || {
                    let primer_dia = primer_dia_del_mes(mes.get());
                    let ultimo_dia = ultimo_dia_del_mes(mes.get());
                    let huecos_iniciales = primer_dia.weekday().num_days_from_monday();
                    let hoy = super::util::hoy();

                    let mut celdas = Vec::new();
                    for _ in 0..huecos_iniciales {
                        celdas.push(view! { <div></div> }.into_any());
                    }

                    let mut dia = primer_dia;
                    while dia <= ultimo_dia {
                        let fecha = dia;
                        let eventos = eventos_de(fecha);
                        let tiene_eventos = !eventos.is_empty();
                        let es_hoy = fecha == hoy;
                        celdas.push(
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        let base = "flex min-h-[52px] flex-col items-center gap-1 rounded-sm py-1.5 text-[12.5px] hover:bg-hover";
                                        if es_hoy {
                                            format!("{base} border border-accent text-accent font-bold")
                                        } else {
                                            format!("{base} text-text")
                                        }
                                    }
                                    on:click=move |_| if tiene_eventos { abrir_dia(fecha) }
                                >
                                    <span>{fecha.day()}</span>
                                    <span class="flex gap-[3px]">
                                        {eventos
                                            .iter()
                                            .take(3)
                                            .map(|ev| {
                                                let color = ev.tipo.color();
                                                view! {
                                                    <span
                                                        class="h-[5px] w-[5px] rounded-full"
                                                        style=format!("background:{color};")
                                                    ></span>
                                                }
                                            })
                                            .collect_view()}
                                    </span>
                                </button>
                            }
                            .into_any(),
                        );
                        dia = dia
                            .checked_add_days(Days::new(1))
                            .expect("nunca se desborda dentro de un mismo mes calendario");
                    }
                    celdas.collect_view()
                }}
            </div>

            <div class="mt-4 flex flex-wrap gap-4 text-[12px] text-muted">
                <span class="flex items-center gap-1.5">
                    <span class="h-[8px] w-[8px] rounded-full" style="background:var(--accent);"></span>
                    "Previstos"
                </span>
                <span class="flex items-center gap-1.5">
                    <span class="h-[8px] w-[8px] rounded-full" style="background:var(--warning);"></span>
                    "Suscripciones"
                </span>
                <span class="flex items-center gap-1.5">
                    <span class="h-[8px] w-[8px] rounded-full" style="background:var(--positive);"></span>
                    "Metas"
                </span>
            </div>
        </div>

        <HojaInferior abierto=hoja_abierta>
            {move || match dia_seleccionado.get() {
                None => ().into_any(),
                Some(fecha) => {
                    let eventos = eventos_de(fecha);
                    view! {
                        <h3 class="mb-4 text-[16px] font-bold text-text">
                            {fecha.format("%d/%m/%Y").to_string()}
                        </h3>
                        <div class="flex flex-col gap-2">
                            {eventos
                                .into_iter()
                                .map(|ev| {
                                    view! {
                                        <div class="flex items-center gap-3 rounded-pane border border-card-line px-4 py-3">
                                            <span
                                                class="h-2 w-2 flex-none rounded-full"
                                                style=format!("background:{};", ev.tipo.color())
                                            ></span>
                                            <div class="min-w-0 flex-1">
                                                <p class="text-[14px] font-semibold text-text">{ev.etiqueta}</p>
                                                <p class="text-[12px] text-faint">{ev.tipo.etiqueta_tipo()}</p>
                                            </div>
                                            {ev.monto.map(|m| view! { <span class="mono text-[14px] font-bold text-text">{format!("{m:.2}")}</span> })}
                                        </div>
                                    }
                                })
                                .collect_view()}
                        </div>
                    }
                    .into_any()
                }
            }}
        </HojaInferior>
    }
}
