//! Cliente HTTP hacia el backend. `client` trae los verbos genéricos
//! (get/post/put/delete); cada submódulo de dominio (`auth`, `accounts`,
//! `accounting`, `tags`...) los usa para exponer funciones tipadas de
//! una sola línea por endpoint.

pub mod accounting;
pub mod accounts;
pub mod admin;
pub mod agenda;
pub mod analytics;
pub mod auth;
mod client;
mod error;
pub mod goals;
pub mod investments;
pub mod movimientos;
pub mod reminders;
pub mod tags;

pub use error::ApiError;
