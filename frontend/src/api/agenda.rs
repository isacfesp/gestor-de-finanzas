//! Llamadas a suscripciones y presupuestos (`/workspaces/:id/suscripciones`,
//! `/presupuestos`, backend `accounting`) y previstos
//! (`/workspaces/:id/previstos`, backend `planned_transactions`) — sección
//! "Agenda" de `docs/frontend-ia.md`. Los structs reflejan 1:1 los de
//! `backend/src/accounting/models.rs` y
//! `backend/src/planned_transactions/models.rs`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::client;
use super::error::ApiError;

// ------------------------------ Suscripciones ------------------------------

/// `periodicity` es un `String` validado por el backend contra estos 4
/// valores exactos — no existe como enum serde, se replica igual aquí.
pub const PERIODICIDADES: [(&str, &str); 4] = [
    ("monthly", "Mensual"),
    ("bimonthly", "Bimestral"),
    ("quarterly", "Trimestral"),
    ("annual", "Anual"),
];

pub fn etiqueta_periodicidad(valor: &str) -> &'static str {
    PERIODICIDADES
        .iter()
        .find(|(v, _)| *v == valor)
        .map(|(_, etiqueta)| *etiqueta)
        .unwrap_or("Desconocida")
}

#[derive(Debug, Clone, Deserialize)]
pub struct Suscripcion {
    pub id: Uuid,
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
    pub is_active: bool,
    /// Cuenta de la que sale el cobro — si está presente, "marcar
    /// cobrada" genera el gasto real (ajusta saldo + transacción); si
    /// no, solo avanza `next_billing_date` (comportamiento previo).
    pub account_id: Option<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct DatosSuscripcion<'a> {
    pub name: &'a str,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: &'a str,
    pub next_billing_date: NaiveDate,
    pub account_id: Option<Uuid>,
}

/// Reemplazo completo (PUT); incluye `is_active` porque el mismo endpoint
/// sirve tanto para editar los datos como para activar/desactivar.
#[derive(Debug, Serialize)]
pub struct ActualizarSuscripcionDatos<'a> {
    pub name: &'a str,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: &'a str,
    pub next_billing_date: NaiveDate,
    pub is_active: bool,
    pub account_id: Option<Uuid>,
}

/// GET /workspaces/:workspace_id/suscripciones — trae activas e inactivas.
pub async fn listar_suscripciones(
    workspace_id: Uuid,
    token: &str,
) -> Result<Vec<Suscripcion>, ApiError> {
    client::get(&format!("/workspaces/{workspace_id}/suscripciones"), token).await
}

/// GET /workspaces/:workspace_id/suscripciones/proximos-cobros?dias=
pub async fn proximos_cobros(
    workspace_id: Uuid,
    dias: i64,
    token: &str,
) -> Result<Vec<Suscripcion>, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/suscripciones/proximos-cobros?dias={dias}"),
        token,
    )
    .await
}

/// POST /workspaces/:workspace_id/suscripciones
pub async fn crear_suscripcion(
    workspace_id: Uuid,
    datos: &DatosSuscripcion<'_>,
    token: &str,
) -> Result<Suscripcion, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/suscripciones"),
        datos,
        token,
    )
    .await
}

/// PUT /workspaces/:workspace_id/suscripciones/:id
pub async fn actualizar_suscripcion(
    workspace_id: Uuid,
    id: Uuid,
    datos: &ActualizarSuscripcionDatos<'_>,
    token: &str,
) -> Result<Suscripcion, ApiError> {
    client::put(
        &format!("/workspaces/{workspace_id}/suscripciones/{id}"),
        datos,
        token,
    )
    .await
}

/// POST /workspaces/:workspace_id/suscripciones/:id/marcar-cobrada —
/// avanza `next_billing_date` según la periodicidad.
pub async fn marcar_cobrada(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<Suscripcion, ApiError> {
    client::post_vacio(
        &format!("/workspaces/{workspace_id}/suscripciones/{id}/marcar-cobrada"),
        token,
    )
    .await
}

// ------------------------------- Presupuestos -------------------------------

#[derive(Debug, Serialize)]
pub struct DatosPresupuesto {
    pub category_id: Uuid,
    pub month: NaiveDate,
    pub limit_amount: Decimal,
}

/// Límite vs. gastado, ya con el nombre de categoría resuelto —
/// respuesta de `GET .../presupuestos/estado`.
#[derive(Debug, Clone, Deserialize)]
pub struct EstadoPresupuesto {
    pub id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub limit_amount: Decimal,
    pub spent: Decimal,
    pub percentage: Decimal,
}

/// POST /workspaces/:workspace_id/presupuestos — upsert por
/// `category_id` + `month`: mismo endpoint para crear y para editar el
/// límite de una categoría ya presupuestada. La respuesta (un
/// `Presupuesto` sin `spent`/`percentage`) no se usa: tras guardar se
/// vuelve a pedir `estado_presupuestos` para refrescar esos cálculos.
pub async fn crear_presupuesto(
    workspace_id: Uuid,
    datos: &DatosPresupuesto,
    token: &str,
) -> Result<(), ApiError> {
    client::post::<_, serde_json::Value>(
        &format!("/workspaces/{workspace_id}/presupuestos"),
        datos,
        token,
    )
    .await
    .map(|_| ())
}

/// GET /workspaces/:workspace_id/presupuestos/estado?month=YYYY-MM-DD
pub async fn estado_presupuestos(
    workspace_id: Uuid,
    month: NaiveDate,
    token: &str,
) -> Result<Vec<EstadoPresupuesto>, ApiError> {
    client::get(
        &format!("/workspaces/{workspace_id}/presupuestos/estado?month={month}"),
        token,
    )
    .await
}

/// DELETE /workspaces/:workspace_id/presupuestos/:id
pub async fn eliminar_presupuesto(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<(), ApiError> {
    client::delete(
        &format!("/workspaces/{workspace_id}/presupuestos/{id}"),
        token,
    )
    .await
}

// -------------------------------- Previstos --------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct Previsto {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub description: Option<String>,
    pub is_paid: bool,
}

/// Cuerpo de POST y de PUT — el backend usa el mismo struct para ambos.
#[derive(Debug, Serialize)]
pub struct DatosPrevisto<'a> {
    #[serde(rename = "type")]
    pub tipo: &'a str,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Option<Uuid>,
    pub description: Option<&'a str>,
}

#[derive(Debug, Default)]
pub struct FiltrosPrevistos {
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub pagado: Option<bool>,
}

impl FiltrosPrevistos {
    fn query_string(&self) -> String {
        let mut partes = Vec::new();
        if let Some(desde) = self.desde {
            partes.push(format!("desde={desde}"));
        }
        if let Some(hasta) = self.hasta {
            partes.push(format!("hasta={hasta}"));
        }
        if let Some(pagado) = self.pagado {
            partes.push(format!("pagado={pagado}"));
        }
        if partes.is_empty() {
            String::new()
        } else {
            format!("?{}", partes.join("&"))
        }
    }
}

/// GET /workspaces/:workspace_id/previstos — con filtros opcionales.
pub async fn listar_previstos(
    workspace_id: Uuid,
    filtros: &FiltrosPrevistos,
    token: &str,
) -> Result<Vec<Previsto>, ApiError> {
    let ruta = format!(
        "/workspaces/{workspace_id}/previstos{}",
        filtros.query_string()
    );
    client::get(&ruta, token).await
}

/// POST /workspaces/:workspace_id/previstos
pub async fn crear_previsto(
    workspace_id: Uuid,
    datos: &DatosPrevisto<'_>,
    token: &str,
) -> Result<Previsto, ApiError> {
    client::post(
        &format!("/workspaces/{workspace_id}/previstos"),
        datos,
        token,
    )
    .await
}

/// PUT /workspaces/:workspace_id/previstos/:id
pub async fn actualizar_previsto(
    workspace_id: Uuid,
    id: Uuid,
    datos: &DatosPrevisto<'_>,
    token: &str,
) -> Result<Previsto, ApiError> {
    client::put(
        &format!("/workspaces/{workspace_id}/previstos/{id}"),
        datos,
        token,
    )
    .await
}

/// POST /workspaces/:workspace_id/previstos/:id/marcar-pagado
pub async fn marcar_pagado(
    workspace_id: Uuid,
    id: Uuid,
    token: &str,
) -> Result<Previsto, ApiError> {
    client::post_vacio(
        &format!("/workspaces/{workspace_id}/previstos/{id}/marcar-pagado"),
        token,
    )
    .await
}

/// DELETE /workspaces/:workspace_id/previstos/:id
pub async fn eliminar_previsto(workspace_id: Uuid, id: Uuid, token: &str) -> Result<(), ApiError> {
    client::delete(&format!("/workspaces/{workspace_id}/previstos/{id}"), token).await
}
