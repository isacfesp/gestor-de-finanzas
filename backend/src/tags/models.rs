// =====================================================================
// models.rs — Structs de datos del módulo tags.
// =====================================================================

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Etiqueta libre de un workspace, usada para agrupar transacciones de
/// forma cruzada (distintas categorías, mismo concepto).
#[derive(Debug, Serialize)]
pub struct Etiqueta {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CrearEtiquetaDatos {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AgregarEtiquetaDatos {
    pub tag_id: Uuid,
}
