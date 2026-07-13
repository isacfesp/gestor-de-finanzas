//! Helpers para el menú de engranaje (`.menu-gear`/`.menu-dropdown`) que
//! usan las tablas de Cuentas y Agenda. El menú se monta con
//! `<leptos::portal::Portal>` directo en `<body>` en vez de vivir dentro
//! de `.table-scroll`: si quedara ahí, el `overflow-x: auto` de esa
//! clase fuerza a `overflow-y` a valer `auto` también (regla de CSS
//! Overflow), y en una tabla de pocas filas el menú se recorta antes de
//! alcanzar su alto completo. Al portarlo, hace falta calcular su
//! posición en pantalla a mano (ya no es descendiente del botón).

use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// Botón (`top`, `right` en px desde el borde de la ventana) donde debe
/// aparecer el menú, calculado a partir del botón que lo abrió.
pub type PosicionMenu = RwSignal<(f64, f64)>;

/// Espacio mínimo respecto al borde de la ventana: sin este margen, un
/// menú anclado justo al borde queda pegado a él o, en viewports muy
/// angostos, lo cruza.
const MARGEN_BORDE: f64 = 8.0;

/// Handler de `on:click` del botón de engranaje: calcula la posición del
/// menú a partir del propio botón (`current_target`, no `target`, para
/// no depender de si el clic cayó en el ícono o en el botón) y alterna
/// si está abierto.
///
/// `ancho_estimado` es el ancho (px) del panel que se va a mostrar
/// (ej. 340.0 para `.notif-dropdown`, ~180.0 para un `.menu-dropdown`
/// de pocas palabras). Solo se usa para el cálculo de abajo, no se
/// aplica como estilo — sin él, anclar el menú por el borde derecho
/// (`right`) deja que su borde izquierdo se salga de la pantalla en
/// viewports angostos (el caso típico: la campana de notificaciones en
/// un celular, con la app instalada como PWA).
pub fn abrir_menu(
    ev: leptos::ev::MouseEvent,
    abierto: RwSignal<bool>,
    posicion: PosicionMenu,
    ancho_estimado: f64,
) {
    if let Some(boton) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let rect = boton.get_bounding_client_rect();
        let ancho_ventana = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // Tope: más allá de este `right`, el borde izquierdo del panel
        // (ancho_ventana - right - ancho_estimado) quedaría por debajo
        // del margen mínimo.
        let right_tope = (ancho_ventana - ancho_estimado - MARGEN_BORDE).max(MARGEN_BORDE);
        let right = (ancho_ventana - rect.right())
            .max(MARGEN_BORDE)
            .min(right_tope);
        posicion.set((rect.bottom() + 4.0, right));
    }
    abierto.update(|v| *v = !*v);
}

/// Estilo inline `position: fixed` con la posición calculada por
/// `abrir_menu`, para aplicar directamente al `.menu-dropdown` portado.
pub fn estilo_posicion(posicion: PosicionMenu) -> String {
    let (top, right) = posicion.get();
    format!("top:{top}px; right:{right}px;")
}
