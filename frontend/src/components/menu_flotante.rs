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

/// Handler de `on:click` del botón de engranaje: calcula la posición del
/// menú a partir del propio botón (`current_target`, no `target`, para
/// no depender de si el clic cayó en el ícono o en el botón) y alterna
/// si está abierto.
pub fn abrir_menu(ev: leptos::ev::MouseEvent, abierto: RwSignal<bool>, posicion: PosicionMenu) {
    if let Some(boton) = ev
        .current_target()
        .and_then(|t| t.dyn_into::<web_sys::Element>().ok())
    {
        let rect = boton.get_bounding_client_rect();
        let ancho_ventana = web_sys::window()
            .and_then(|w| w.inner_width().ok())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        posicion.set((rect.bottom() + 4.0, ancho_ventana - rect.right()));
    }
    abierto.update(|v| *v = !*v);
}

/// Estilo inline `position: fixed` con la posición calculada por
/// `abrir_menu`, para aplicar directamente al `.menu-dropdown` portado.
pub fn estilo_posicion(posicion: PosicionMenu) -> String {
    let (top, right) = posicion.get();
    format!("top:{top}px; right:{right}px;")
}
