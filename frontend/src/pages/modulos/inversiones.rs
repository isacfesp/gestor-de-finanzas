use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Sección "Inversiones" de `docs/frontend-ia.md`: metas de ahorro
/// (backend: `goals`) e inversiones SOFIPO con rendimiento/ISR/simulador
/// (backend: `investments`) — se agrupan por ser "dinero apartado para
/// el futuro", a diferencia del flujo de caja del día a día en Cuentas.
#[component]
pub fn InversionesPage() -> impl IntoView {
    view! { <Placeholder titulo="Inversiones" descripcion="Metas de ahorro, inversiones, rendimiento e ISR — en construcción."/> }
}
