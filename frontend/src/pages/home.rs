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
mod tasa_ahorro;
mod tendencia;
mod util;

use accesos_rapidos::AccesosRapidos;
use auditoria_rapida::AuditoriaRapida;
use distribucion::Distribucion;
use kpis::Kpis;
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
            <div style="margin-top:16px;">
                <Kpis workspace_id=workspace.id().unwrap_or(Uuid::nil()) desde=desde hasta=hasta/>
            </div>
            <TasaAhorro workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            <Tendencia workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            <Distribucion workspace_id=workspace.id().unwrap_or(Uuid::nil()) desde=desde hasta=hasta/>
            <AccesosRapidos workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
            <AuditoriaRapida workspace_id=workspace.id().unwrap_or(Uuid::nil())/>
        </Show>
    }
}
