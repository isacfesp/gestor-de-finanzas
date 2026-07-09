//! Llamadas a `/workspaces/:workspace_id/categorias` y `/transacciones`.
//! Los structs reflejan 1:1 los de `backend/src/accounting/models.rs`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

// ------------------------------- Categorías -------------------------------

/// `workspace_id` es `None` para una categoría global (visible desde
/// cualquier workspace, no se puede borrar) y `Some` para una propia.
#[derive(Debug, Clone, Deserialize)]
pub struct Categoria {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
}

#[derive(Debug, Serialize)]
pub struct CrearCategoriaDatos<'a> {
    pub name: &'a str,
    #[serde(rename = "type")]
    pub tipo: &'a str,
}

/// GET /workspaces/:workspace_id/categorias
pub async fn listar_categorias(
    workspace_id: Uuid,
    token: &str,
) -> Result<Vec<Categoria>, ApiError> {
    client::get(&format!("/workspaces/{workspace_id}/categorias"), token).await
}

/// POST /workspaces/:workspace_id/categorias
pub async fn crear_categoria(
    workspace_id: Uuid,
    datos: &CrearCategoriaDatos<'_>,
    token: &str,
) -> Result<Categoria, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/categorias"),
        datos,
        token,
    )
    .await
}

/// DELETE /workspaces/:workspace_id/categorias/:id — solo categorías
/// propias; 409 si está en uso (transacciones, suscripciones, etc.).
pub async fn eliminar_categoria(workspace_id: Uuid, id: Uuid, token: &str) -> Result<(), ApiError> {
    client::delete(
        &format!("/workspaces/{workspace_id}/categorias/{id}"),
        token,
    )
    .await
}

// ------------------------------ Transacciones ------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Transaccion {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Uuid,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DatosTransaccion<'a> {
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Uuid,
    pub description: Option<&'a str>,
}

/// Filtros opcionales de `listar_transacciones`. `None` en un campo
/// significa "no filtrar por esto" (se omite de la query string).
#[derive(Debug, Default)]
pub struct FiltrosTransacciones<'a> {
    pub tipo: Option<&'a str>,
    pub category_id: Option<Uuid>,
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
}

impl FiltrosTransacciones<'_> {
    /// Arma la query string (`?type=...&category_id=...`) a partir de
    /// los campos presentes. Devuelve "" si no hay ningún filtro.
    fn query_string(&self) -> String {
        let mut partes = Vec::new();
        if let Some(tipo) = self.tipo {
            partes.push(format!("type={tipo}"));
        }
        if let Some(category_id) = self.category_id {
            partes.push(format!("category_id={category_id}"));
        }
        if let Some(desde) = self.desde {
            partes.push(format!("desde={desde}"));
        }
        if let Some(hasta) = self.hasta {
            partes.push(format!("hasta={hasta}"));
        }
        if partes.is_empty() {
            String::new()
        } else {
            format!("?{}", partes.join("&"))
        }
    }
}

/// GET /workspaces/:workspace_id/transacciones — con filtros opcionales.
pub async fn listar_transacciones(
    workspace_id: Uuid,
    filtros: &FiltrosTransacciones<'_>,
    token: &str,
) -> Result<Vec<Transaccion>, ApiError> {
    let ruta = format!(
        "/workspaces/{workspace_id}/transacciones{}",
        filtros.query_string()
    );
    client::get(&ruta, token).await
}

/// POST /workspaces/:workspace_id/transacciones
pub async fn crear_transaccion(
    workspace_id: Uuid,
    datos: &DatosTransaccion<'_>,
    token: &str,
) -> Result<Transaccion, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/transacciones"),
        datos,
        token,
    )
    .await
}

/// PUT /workspaces/:workspace_id/transacciones/:id
pub async fn actualizar_transaccion(
    workspace_id: Uuid,
    id: Uuid,
    datos: &DatosTransaccion<'_>,
    token: &str,
) -> Result<Transaccion, ApiError> {
    client::put(
        &format!("/workspaces/{workspace_id}/transacciones/{id}"),
        datos,
        token,
    )
    .await
}

/// DELETE /workspaces/:workspace_id/transacciones/:id
pub async fn eliminar_transaccion(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<(), ApiError> {
    client::delete(
        &format!("/workspaces/{workspace_id}/transacciones/{id}"),
        token,
    )
    .await
}
