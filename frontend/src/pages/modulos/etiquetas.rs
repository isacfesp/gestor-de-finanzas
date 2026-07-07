use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Corresponde al módulo `tags` del backend (etiquetas libres y su
/// asociación con transacciones).
#[component]
pub fn EtiquetasPage() -> impl IntoView {
    view! { <Placeholder titulo="Etiquetas" descripcion="Etiquetas y su relación con movimientos — en construcción."/> }
}
