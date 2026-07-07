use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Corresponde al módulo `accounts` del backend (cuentas/billeteras y
/// transferencias entre ellas).
#[component]
pub fn CuentasPage() -> impl IntoView {
    view! { <Placeholder titulo="Cuentas" descripcion="Cuentas, billeteras y transferencias — en construcción."/> }
}
