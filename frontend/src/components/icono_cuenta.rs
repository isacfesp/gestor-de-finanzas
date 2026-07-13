//! Ícono según el tipo de cuenta (`cash`/`debit`/`credit`/`savings`/
//! `investment` — mismos valores que `accounts::Cuenta.tipo`). Antes
//! las cuentas solo se distinguían por texto (`etiqueta_tipo`); en
//! tarjetas (uso típico en móvil) un ícono se reconoce más rápido que
//! leer la palabra completa.

use leptos::prelude::*;

fn trazo(tipo: &str) -> AnyView {
    match tipo {
        "cash" => view! {
            // Billete: rectángulo apaisado con un óvalo al centro.
            <rect x="2.5" y="6" width="19" height="12" rx="2"></rect>
            <circle cx="12" cy="12" r="2.6"></circle>
            <path d="M5.5 9v0M18.5 15v0"></path>
        }
        .into_any(),
        "credit" => view! {
            // Tarjeta con banda de firma (segunda línea, distinta de
            // la banda magnética de "debit") y un chip.
            <rect x="2.5" y="5" width="19" height="14" rx="2"></rect>
            <path d="M2.5 10h19"></path>
            <rect x="5" y="13.5" width="4" height="2.6" rx="0.6"></rect>
        }
        .into_any(),
        "savings" => view! {
            // Alcancía: cuerpo redondeado, ranura y patitas.
            <path d="M4 13a6.5 6.5 0 0 1 6.5-6.5H15a5 5 0 0 1 5 5v.5a5 5 0 0 1-5 5H10a6 6 0 0 1-6-5.5Z"></path>
            <path d="M10.5 9.5h2.2M7 18v1.6M15 18v1.6"></path>
            <circle cx="16.2" cy="11.5" r="0.8"></circle>
        }
        .into_any(),
        "investment" => view! {
            // Tendencia ascendente — mismo trazo que el ícono de
            // "Inversiones" en el rail de navegación (shell.rs), para
            // que la asociación visual sea consistente en toda la app.
            <path d="M4 16l5-5 4 4 7-8"></path>
            <path d="M15 7h5v5"></path>
        }
        .into_any(),
        // "debit" y cualquier tipo no reconocido caen en la tarjeta
        // simple (banda magnética, sin chip).
        _ => view! {
            <rect x="2.5" y="5" width="19" height="14" rx="2"></rect>
            <path d="M2.5 10h19"></path>
        }
        .into_any(),
    }
}

#[component]
pub fn IconoCuenta(tipo: String, #[prop(optional, into)] class: String) -> impl IntoView {
    let clase = if class.is_empty() {
        "h-[18px] w-[18px] flex-none fill-none stroke-current [stroke-linecap:round] [stroke-linejoin:round] [stroke-width:1.7]".to_string()
    } else {
        class
    };
    view! {
        <svg viewBox="0 0 24 24" class=clase>
            {trazo(&tipo)}
        </svg>
    }
}
