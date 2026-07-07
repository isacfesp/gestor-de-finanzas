use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Corresponde al módulo `accounting` del backend (transacciones,
/// categorías, suscripciones, presupuestos).
#[component]
pub fn MovimientosPage() -> impl IntoView {
    view! { <Placeholder titulo="Movimientos" descripcion="Transacciones, presupuestos y suscripciones — en construcción."/> }
}
