//! Helpers compartidos entre los widgets del Dashboard.

use chrono::{Datelike, NaiveDate};

/// Fecha de hoy según el reloj del navegador.
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

/// Primer día del mes en curso, para prellenar los filtros de período.
pub fn primer_dia_mes() -> NaiveDate {
    let h = hoy();
    NaiveDate::from_ymd_opt(h.year(), h.month(), 1).expect("el día 1 siempre es válido")
}
