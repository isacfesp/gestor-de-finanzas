use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Sección "Agenda" de `docs/frontend-ia.md`: gastos fijos/suscripciones,
/// presupuestos (por periodo o por meta) y pagos/ingresos previstos —
/// backend: `accounting` (subscriptions, budgets) y `planned_transactions`.
#[component]
pub fn AgendaPage() -> impl IntoView {
    view! { <Placeholder titulo="Agenda" descripcion="Suscripciones, presupuestos y previstos — en construcción."/> }
}
