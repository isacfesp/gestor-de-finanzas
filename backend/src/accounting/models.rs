// =====================================================================
// models.rs — Structs de datos del módulo accounting.
//
// La columna `type` existe en varias tablas pero `type` es palabra
// reservada en Rust; por eso el campo se llama `tipo` y se renombra
// con #[serde(rename = "type")] para que la API siga hablando en los
// mismos términos que la base de datos.
// =====================================================================

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ------------------------------- Categorías -------------------------------

/// Categoría de transacción. `workspace_id` es None para las categorías
/// globales (visibles desde cualquier workspace).
#[derive(Debug, Serialize)]
pub struct Categoria {
    pub id: Uuid,
    pub workspace_id: Option<Uuid>,
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
}

#[derive(Debug, Deserialize)]
pub struct CrearCategoriaDatos {
    pub name: String,
    #[serde(rename = "type")]
    pub tipo: String,
}

// ------------------------------ Transacciones ------------------------------

#[derive(Debug, Serialize)]
pub struct Transaccion {
    pub id: Uuid,
    pub workspace_id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Se usa tanto para crear como para reemplazar una transacción
/// existente (PUT): en ambos casos se exigen todos los campos.
#[derive(Debug, Deserialize)]
pub struct DatosTransaccion {
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosTransacciones {
    #[serde(rename = "type")]
    pub tipo: Option<String>,
    pub category_id: Option<Uuid>,
    pub desde: Option<NaiveDate>,
    pub hasta: Option<NaiveDate>,
    pub tag_id: Option<Uuid>,
    pub limite: Option<i64>,
    pub desplazamiento: Option<i64>,
}

// ------------------------------ Suscripciones ------------------------------

#[derive(Debug, Serialize)]
pub struct Suscripcion {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CrearSuscripcionDatos {
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct ActualizarSuscripcionDatos {
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosSuscripciones {
    pub activas: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct FiltroProximosCobros {
    /// Ventana de días hacia adelante a considerar (por defecto 30).
    pub dias: Option<i64>,
}

// ------------------------------ Presupuestos ------------------------------

#[derive(Debug, Serialize)]
pub struct Presupuesto {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub category_id: Uuid,
    pub month: NaiveDate,
    pub limit_amount: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CrearPresupuestoDatos {
    pub category_id: Uuid,
    pub month: NaiveDate,
    pub limit_amount: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct FiltroMes {
    pub month: Option<NaiveDate>,
}

/// Estado de un presupuesto: límite vs. lo realmente gastado en el mes.
#[derive(Debug, Serialize)]
pub struct EstadoPresupuesto {
    pub id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub month: NaiveDate,
    pub limit_amount: Decimal,
    pub spent: Decimal,
    /// Porcentaje consumido del límite (100 = justo al límite).
    pub percentage: Decimal,
}
