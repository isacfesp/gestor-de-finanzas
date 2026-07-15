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
    /// Cuenta de la que sale (gasto) o a la que entra (ingreso) el
    /// dinero. Crear/editar/borrar una transacción ajusta el saldo de
    /// esta cuenta (ver `transacciones.rs`).
    pub account_id: Uuid,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Fila de listado: igual que `Transaccion` pero con el nombre de
/// quién la registró (JOIN con `users`, mismo patrón que
/// `goals::Aporte`) y el nombre/tipo de la cuenta (JOIN con
/// `accounts`) — desde que las cuentas son personales
/// (`accounts::Cuenta::owner_id`), un miembro solo recibe SUS propias
/// cuentas de `GET .../cuentas`, así que ya no puede resolver el
/// nombre de una cuenta ajena cruzando esa lista en el frontend; el
/// listado de movimientos, que sigue siendo de todo el workspace, debe
/// traer el nombre ya resuelto. Vive aparte de `Transaccion` porque
/// crear/editar no necesitan estos JOIN.
#[derive(Debug, Serialize)]
pub struct TransaccionListado {
    pub id: Uuid,
    pub workspace_id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub category_id: Option<Uuid>,
    pub account_id: Uuid,
    pub account_name: String,
    pub account_tipo: String,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_by_name: String,
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
    pub account_id: Uuid,
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
    /// Dueño individual: solo él la crea/edita/marca cobrada/elimina.
    pub owner_id: Uuid,
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    /// Cuenta de la que sale el cobro — opcional (una suscripción sin
    /// cuenta sigue siendo válida, solo que "marcar cobrada" no genera
    /// movimiento real, ver `suscripciones::marcar_cobrada`).
    pub account_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CrearSuscripcionDatos {
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
    pub account_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ActualizarSuscripcionDatos {
    pub name: String,
    pub amount: Decimal,
    pub category_id: Option<Uuid>,
    pub periodicity: String,
    pub next_billing_date: NaiveDate,
    pub is_active: bool,
    pub account_id: Option<Uuid>,
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
    /// Dueño individual: cada usuario tiene su propio límite por
    /// categoría/mes (la categoría en sí sigue siendo compartida).
    pub owner_id: Uuid,
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
    pub owner_id: Uuid,
    pub category_id: Uuid,
    pub category_name: String,
    pub month: NaiveDate,
    pub limit_amount: Decimal,
    pub spent: Decimal,
    /// Porcentaje consumido del límite (100 = justo al límite).
    pub percentage: Decimal,
}
