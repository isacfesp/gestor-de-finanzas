// =====================================================================
// comun.rs — Regla de visibilidad compartida por las 5 métricas.
// =====================================================================

use uuid::Uuid;

use crate::auth::extractores::UsuarioAutenticado;

/// A qué `created_by` filtrar una métrica.
///
/// - Un usuario normal siempre ve solo lo suyo: se ignora cualquier
///   `user_id` que haya mandado y se fuerza a sí mismo.
/// - Un dev sin `user_id` ve todo el workspace (`None` = sin filtro).
/// - Un dev con `user_id` inspecciona a ese usuario en particular.
pub(crate) fn resolver_filtro_usuario(
    usuario: &UsuarioAutenticado,
    user_id: Option<Uuid>,
) -> Option<Uuid> {
    if usuario.es_dev() {
        user_id
    } else {
        Some(usuario.id)
    }
}
