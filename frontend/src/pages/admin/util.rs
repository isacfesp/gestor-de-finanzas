//! Helpers compartidos entre las pestañas del panel de administración.

/// Pregunta de confirmación nativa del navegador para acciones
/// destructivas (eliminar/desactivar). Mismo patrón que el resto del
/// proyecto (ver `pages/modulos/agenda/util.rs`).
pub fn confirmar(mensaje: &str) -> bool {
    web_sys::window()
        .and_then(|w| w.confirm_with_message(mensaje).ok())
        .unwrap_or(false)
}
