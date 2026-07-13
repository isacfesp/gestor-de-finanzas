//! Hoja inferior (bottom sheet): panel que entra desde abajo con
//! backdrop y gesto de arrastrar-para-cerrar en el handle superior.
//! Sirve como contenedor genérico para lo que en móvil abre `BotonRapido`
//! y, más adelante, los formularios de crear/editar que hoy son
//! paneles inline (`.form-panel`) — ver plan de rediseño mobile-first.

use leptos::ev::PointerEvent;
use leptos::portal::Portal;
use leptos::prelude::*;

/// Distancia de arrastre (px) a partir de la cual soltar cierra la
/// hoja — por debajo de este umbral, vuelve a su posición: no todo
/// gesto que toca el handle es una intención de cerrar.
const UMBRAL_CIERRE_PX: f64 = 90.0;

#[component]
pub fn HojaInferior(
    /// Controla si la hoja está montada/visible. El caller es dueño del
    /// signal (lo crea y lo usa también para abrirla).
    abierto: RwSignal<bool>,
    // `ChildrenFn` (no `Children`): el contenido se puede volver a
    // pintar cada vez que `abierto` pasa a `true` dentro de `<Show>`,
    // que llama al closure más de una vez a lo largo de la vida de la
    // hoja — `Children` es de un solo uso (`FnOnce`) y no alcanza.
    children: ChildrenFn,
) -> impl IntoView {
    // `<Show>` reconstruye el `<Portal>` de abajo cada vez que `abierto`
    // vuelve a `true`, y esa reconstrucción necesita mover `children`
    // (un `Arc`, no copiable) hacia el nuevo closure — al segundo ciclo
    // ya no quedaría nada que mover. `StoredValue` sí es `Copy` (es un
    // handle a un valor guardado aparte), así que cada reconstrucción
    // puede "moverlo" sin agotar nada; `.get_value()` clona el `Arc` de
    // adentro, que es barato.
    let children = StoredValue::new(children);
    // Desplazamiento vertical actual del arrastre (px). Vive en un
    // signal — igual que el resto de la reactividad de Leptos — para
    // que el `style` de abajo se recalcule solo en cada `pointermove`,
    // sin tocar el DOM a mano vía `web_sys::window()...`.
    let arrastre_y = RwSignal::new(0.0_f64);
    // Y de pantalla donde empezó el gesto actual; `None` cuando no se
    // está arrastrando (lo usan `on_mover`/`on_soltar` para saber si el
    // puntero que se movió es el mismo que bajó en el handle).
    let inicio_y = RwSignal::new(None::<f64>);

    let on_bajar = move |ev: PointerEvent| {
        inicio_y.set(Some(ev.client_y() as f64));
    };
    let on_mover = move |ev: PointerEvent| {
        if let Some(inicio) = inicio_y.get_untracked() {
            let delta = (ev.client_y() as f64 - inicio).max(0.0);
            arrastre_y.set(delta);
        }
    };
    let on_soltar = move |_: PointerEvent| {
        if inicio_y.get_untracked().is_some() {
            if arrastre_y.get_untracked() > UMBRAL_CIERRE_PX {
                abierto.set(false);
            }
            inicio_y.set(None);
            arrastre_y.set(0.0);
        }
    };

    view! {
        <Show when=move || abierto.get()>
            <Portal>
                <div
                    class="fixed inset-0 z-30 bg-black/50"
                    on:click=move |_| abierto.set(false)
                ></div>
                <div
                    class="fixed inset-x-0 bottom-0 z-30 max-h-[85dvh] overflow-y-auto rounded-t-[20px] border-t border-card-line bg-panel pb-[env(safe-area-inset-bottom)] shadow-[0_-12px_30px_-10px_rgba(0,0,0,.35)]"
                    style=move || {
                        let arrastrando = inicio_y.get().is_some();
                        format!(
                            "transform: translateY({}px); transition: {};",
                            arrastre_y.get(),
                            if arrastrando { "none" } else { "transform .2s ease" },
                        )
                    }
                >
                    <div
                        class="flex touch-none justify-center py-2.5"
                        on:pointerdown=on_bajar
                        on:pointermove=on_mover
                        on:pointerup=on_soltar
                        on:pointercancel=on_soltar
                    >
                        <span class="h-1.5 w-10 rounded-full bg-card-line"></span>
                    </div>
                    <div class="px-5 pb-6">{move || children.get_value()()}</div>
                </div>
            </Portal>
        </Show>
    }
}
