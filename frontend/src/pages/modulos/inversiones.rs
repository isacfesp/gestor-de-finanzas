use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Corresponde al módulo `investments` del backend (inversiones,
/// rendimiento, ISR y simulador).
#[component]
pub fn InversionesPage() -> impl IntoView {
    view! { <Placeholder titulo="Inversiones" descripcion="Inversiones, rendimiento e ISR — en construcción."/> }
}
