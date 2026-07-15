//! Dashboard: saludo + indicadores de `analytics` (flujo de caja, tasa
//! de ahorro, distribución de gastos), accesos rápidos a transacciones
//! recientes/próximos eventos, y una auditoría rápida del workspace
//! (ver `docs/frontend-ia.md`).

use leptos::prelude::*;
use uuid::Uuid;

use crate::auth::use_auth;
use crate::workspace::use_workspace;

mod accesos_rapidos;
mod auditoria_rapida;
mod distribucion;
mod kpis;
mod selector_alcance;
mod tasa_ahorro;
mod tendencia;
mod util;

use accesos_rapidos::AccesosRapidos;
use auditoria_rapida::AuditoriaRapida;
use distribucion::Distribucion;
use kpis::Kpis;
use selector_alcance::SelectorAlcance;
use tasa_ahorro::TasaAhorro;
use tendencia::Tendencia;

#[component]
pub fn Home() -> impl IntoView {
    let auth = use_auth();
    let workspace = use_workspace();

    // Filtro de período compartido entre el flujo de caja y la
    // distribución de gastos — ambos son "métricas de un rango de
    // fechas", a diferencia de la tasa de ahorro, que la spec ata al
    // mes en curso (ver TasaAhorro).
    let desde = RwSignal::new(String::new());
    let hasta = RwSignal::new(String::new());
    // Solo un dev global puede pedir métricas de otro usuario o del
    // workspace completo; para cualquier otro se ignora en el backend
    // y este signal nunca se toca (ver `SelectorAlcance`).
    let alcance = RwSignal::new(None::<Uuid>);

    view! {
        <section class="panel" style="padding: 22px 20px;">
            <p class="eyebrow">"Sesión iniciada"</p>
            {move || {
                auth.usuario()
                    .map(|u| {
                        view! {
                            <p class="figure" style="font-size: 24px; margin: 6px 0 4px;">
                                "Hola, " {u.name}
                            </p>
                            <p class="text-soft">{u.email} " · rol " {u.role}</p>
                        }
                            .into_any()
                    })
                    .unwrap_or_else(|| view! { <p class="text-soft">"Cargando..."</p> }.into_any())
            }}
        </section>

        <Show
            when=move || workspace.id().is_some()
            fallback=move || {
                view! {
                    <section class="panel" style="margin-top:16px;">
                        <p class="text-soft">
                            {move || workspace.error().unwrap_or_else(|| "Cargando workspace...".to_string())}
                        </p>
                    </section>
                }
            }
        >
            <Show when=move || auth.es_dev()>
                <div style="margin-top:16px;">
                    <SelectorAlcance workspace_id=workspace.id().unwrap_or(Uuid::nil()) alcance=alcance/>
                </div>
            </Show>
            <div style="margin-top:16px;">
                <Kpis workspace_id=workspace.id().unwrap_or(Uuid::nil()) desde=desde hasta=hasta alcance=alcance/>
            </div>
            <TasaAhorro workspace_id=workspace.id().unwrap_or(Uuid::nil()) alcance=alcance/>
            <Tendencia workspace_id=workspace.id().unwrap_or(Uuid::nil()) alcance=alcance/>
            <Distribucion workspace_id=workspace.id().unwrap_or(Uuid::nil()) desde=desde hasta=hasta alcance=alcance/>
            <AccesosRapidos workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            <AuditoriaRapida workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
        </Show>
    }
}
