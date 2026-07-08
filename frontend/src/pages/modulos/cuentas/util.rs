//! Helpers compartidos entre las dos pestañas del módulo Cuentas.

use chrono::NaiveDate;

/// Fecha de hoy según el reloj del navegador, para prellenar los
/// formularios de transacción/transferencia.
pub fn hoy() -> NaiveDate {
    let ahora = js_sys::Date::new_0();
    // Invariante imposible de romper: el reloj del navegador siempre
    // reporta una fecha calendario válida.
    NaiveDate::from_ymd_opt(
        ahora.get_full_year() as i32,
        ahora.get_month() + 1,
        ahora.get_date(),
    )
    .expect("la fecha del sistema siempre es válida")
}

/// Pregunta de confirmación nativa del navegador para acciones
/// destructivas (borrar). Si por algún motivo no hay `window` (no
/// debería pasar en un SPA), se asume que no se confirmó — más seguro
/// que asumir que sí.
pub fn confirmar(mensaje: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(mensaje).ok())
        .unwrap_or(false)
}
