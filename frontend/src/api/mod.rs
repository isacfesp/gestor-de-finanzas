//! Cliente HTTP hacia el backend. `client` trae los verbos genéricos
//! (get/post/put/delete); cada submódulo de dominio (`auth`, `accounts`,
//! `accounting`, `tags`...) los usa para exponer funciones tipadas de
//! una sola línea por endpoint.

pub mod accounting;
pub mod accounts;
pub mod admin;
pub mod auth;
mod client;
mod error;
pub mod tags;

pub use error::ApiError;
