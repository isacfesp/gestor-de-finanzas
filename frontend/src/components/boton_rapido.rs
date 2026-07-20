//! Botón de acceso rápido (FAB) en móvil: un tap y medio (FAB → acción)
//! en vez de recorrer el rail de navegación y el módulo correspondiente
//! para llegar a "nueva transacción" — ver plan de rediseño mobile-first
//! ("reducir los clics al máximo"). Cada acción no solo navega al
//! módulo: aterriza directo en la pestaña correcta con el formulario de
//! creación ya desplegado (query params `?tab=&crear=1`, leídos por
//! cada página en `pages::modulos::*`), y el primer campo enfocado para
//! que en mobile salga el teclado de una vez (ver `node_ref`/`on_load`
//! en `FormularioOperacion`/`FormularioPrevisto`/`FormularioAporte`).
//! "Aportar a una meta" no trae un `goal_id` (el FAB no sabe a cuál):
//! si hay una sola meta activa, `PestanaMetas` la elige sola; si no,
//! aterriza en la lista (o en "nueva meta" si todavía no hay ninguna).

use leptos::prelude::*;
use leptos_router::components::A;

use crate::components::hoja_inferior::HojaInferior;

const ACCION_CLASS: &str = "flex items-center gap-3 rounded-pane border border-card-line \
    px-4 py-3.5 text-[14px] font-semibold text-text transition-colors hover:bg-hover";

const ACCIONES: &[(&str, &str, &str)] = &[
    (
        "Nueva transacción",
        "/cuentas?tab=transacciones&crear=1",
        "wallet",
    ),
    (
        "Nuevo previsto",
        "/agenda?tab=previstos&crear=1",
        "calendar",
    ),
    (
        "Aportar a una meta",
        "/inversiones?tab=metas&aportar=1",
        "trend",
    ),
];

fn icono(kind: &str) -> AnyView {
    match kind {
        "wallet" => view! {
            <rect x="3.5" y="6" width="17" height="12" rx="2"></rect><path d="M3.5 10h17"></path><circle cx="16.5" cy="14" r="1"></circle>
        }
        .into_any(),
        "trend" => view! {
            <path d="M4 16l5-5 4 4 7-8"></path><path d="M15 7h5v5"></path>
        }
        .into_any(),
        _ => view! {
            <rect x="4" y="5" width="16" height="15" rx="2"></rect><path d="M4 9h16M8 3v4M16 3v4"></path>
        }
        .into_any(),
    }
}

#[component]
pub fn BotonRapido() -> impl IntoView {
    let abierto = RwSignal::new(false);

    view! {
        <button
            type="button"
            class="fixed bottom-[calc(76px+env(safe-area-inset-bottom))] right-4 z-20 flex h-14 w-14 items-center justify-center rounded-full bg-[linear-gradient(135deg,var(--accent-2),var(--accent))] text-[#04222e] shadow-[0_10px_26px_-8px_var(--accent)] md:hidden"
            title="Acceso rápido"
            on:click=move |_| abierto.set(true)
        >
            <svg viewBox="0 0 24 24" class="h-6 w-6 fill-none stroke-current [stroke-linecap:round] [stroke-width:2]">
                <path d="M12 5v14M5 12h14"></path>
            </svg>
        </button>
        <HojaInferior abierto=abierto>
            <h3 class="mb-4 text-[16px] font-bold text-text">"Acceso rápido"</h3>
            <div class="flex flex-col gap-2">
                {ACCIONES
                    .iter()
                    .map(|(label, path, kind)| {
                        view! {
                            <A href=*path attr:class=ACCION_CLASS on:click=move |_| abierto.set(false)>
                                <svg viewBox="0 0 24 24" class="h-5 w-5 flex-none fill-none stroke-current [stroke-linecap:round] [stroke-linejoin:round] [stroke-width:1.8]">
                                    {icono(kind)}
                                </svg>
                                <span>{*label}</span>
                            </A>
                        }
                    })
                    .collect_view()}
            </div>
        </HojaInferior>
    }
}
