use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Corresponde al módulo `planned_transactions` del backend (pagos e
/// ingresos previstos).
#[component]
pub fn PrevistosPage() -> impl IntoView {
    view! { <Placeholder titulo="Previstos" descripcion="Pagos e ingresos previstos — en construcción."/> }
}
