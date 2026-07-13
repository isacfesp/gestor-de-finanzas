// =====================================================================
// models.rs — Structs de datos del módulo goals.
// =====================================================================

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Meta de ahorro con su avance actual.
#[derive(Debug, Serialize)]
pub struct Meta {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    pub target_amount: Decimal,
    pub current_amount: Decimal,
    pub deadline: NaiveDate,
    pub is_completed: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CrearMetaDatos {
    pub name: String,
    pub target_amount: Decimal,
    pub deadline: NaiveDate,
}

/// `current_amount` e `is_completed` no se editan aquí: solo cambian
/// al vincular un aporte (ver aportes::vincular).
#[derive(Debug, Deserialize)]
pub struct ActualizarMetaDatos {
    pub name: String,
    pub target_amount: Decimal,
    pub deadline: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct FiltrosMetas {
    pub completadas: Option<bool>,
}

/// Un aporte es, en la práctica, una transacción de ingreso (o un
/// retiro con tipo 'expense') vinculada a la meta.
#[derive(Debug, Deserialize)]
pub struct AporteDatos {
    pub amount: Decimal,
    /// 'income' suma al ahorro, 'expense' lo retira. Por defecto 'income'.
    #[serde(rename = "type")]
    pub tipo: Option<String>,
    pub date: NaiveDate,
    pub description: Option<String>,
    /// Cuenta de la que sale el aporte ('income') o a la que vuelve el
    /// retiro ('expense') — obligatoria: un aporte siempre mueve dinero
    /// real de/hacia una cuenta concreta (ver `aportes::vincular`).
    pub account_id: Uuid,
}

/// Progreso de una meta: saldo restante y porcentaje completado.
#[derive(Debug, Serialize)]
pub struct ProgresoMeta {
    pub id: Uuid,
    pub name: String,
    pub target_amount: Decimal,
    pub current_amount: Decimal,
    pub remaining_amount: Decimal,
    pub percentage: Decimal,
    pub deadline: NaiveDate,
    pub is_completed: bool,
}

#[derive(Debug, Deserialize)]
pub struct FiltroProyeccion {
    /// 'weekly' o 'monthly'. Por defecto 'monthly'.
    pub periodo: Option<String>,
}

/// Cuánto aportar por período (semana o mes) para llegar a la meta a
/// tiempo, según los períodos que quedan hasta `deadline`.
#[derive(Debug, Serialize)]
pub struct ProyeccionMeta {
    pub periodo: String,
    pub periodos_restantes: i64,
    pub aporte_necesario: Decimal,
}

/// Un aporte visto desde afuera: la transacción que lo originó.
#[derive(Debug, Serialize)]
pub struct Aporte {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub tipo: String,
    pub amount: Decimal,
    pub date: NaiveDate,
    pub description: Option<String>,
    /// Nombre de quién registró el aporte — la meta es colaborativa
    /// entre varios miembros del workspace, así que el historial debe
    /// distinguir quién dejó cada monto.
    pub created_by_name: String,
}
