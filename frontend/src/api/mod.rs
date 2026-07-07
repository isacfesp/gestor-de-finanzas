//! Cliente HTTP hacia el backend. `client` trae los verbos genéricos
//! (get/post/put/delete); cada submódulo de dominio (`auth`, y más
//! adelante `accounts`, `goals`, etc.) los usa para exponer funciones
//! tipadas de una sola línea por endpoint.

pub mod auth;
mod client;
mod error;

pub use error::ApiError;
