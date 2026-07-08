//! Una página por sección de navegación, según la arquitectura de
//! información definida en `docs/frontend-ia.md` (no es 1:1 con los
//! módulos del backend — varios backend módulos conviven en una sola
//! pantalla, ej. Cuentas agrupa `accounts` + `accounting` + `tags`).

mod agenda;
mod cuentas;
mod inversiones;
mod movimientos;

pub use agenda::AgendaPage;
pub use cuentas::CuentasPage;
pub use inversiones::InversionesPage;
pub use movimientos::MovimientosPage;
