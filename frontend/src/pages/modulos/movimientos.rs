use leptos::prelude::*;

use crate::pages::placeholder::Placeholder;

/// Sección "Movimientos" de `docs/frontend-ia.md`: auditoría — historial
/// de actividad del workspace (quién hizo qué y cuándo). **No** son las
/// transacciones de ingresos/gastos, esas viven en Cuentas.
///
/// Pendiente: el endpoint actual (`GET /admin/auditoria`) es solo para
/// el rol `dev`; hace falta una versión con alcance de workspace para
/// que un usuario normal vea esta pantalla (ver `docs/frontend-ia.md`).
#[component]
pub fn MovimientosPage() -> impl IntoView {
    view! { <Placeholder titulo="Movimientos" descripcion="Auditoría del workspace — en construcción."/> }
}
