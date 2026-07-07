//! Una página por módulo de negocio, reflejando 1:1 los módulos del
//! backend (ver `backend/src/`). Hoy cada una solo muestra un
//! placeholder; el trabajo de un módulo nuevo empieza reemplazando el
//! cuerpo de su función aquí por la UI real, y agregando sus propias
//! llamadas tipadas en `crate::api`.

mod cuentas;
mod etiquetas;
mod inversiones;
mod metas;
mod movimientos;
mod previstos;

pub use cuentas::CuentasPage;
pub use etiquetas::EtiquetasPage;
pub use inversiones::InversionesPage;
pub use metas::MetasPage;
pub use movimientos::MovimientosPage;
pub use previstos::PrevistosPage;
