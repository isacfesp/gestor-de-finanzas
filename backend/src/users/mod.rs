/*Este archivo actúa como la puerta de entrada de la carpeta.
Aquí declaras qué archivos internos existen y expones la función que arma el sub-router. */

//Ejemplo
/* Declaramos los archivos internos como módulos privados de esta carpeta
mod handlers;
mod models;

use axum::{routing::post, Router};
use sqlx::PgPool; // O MySqlPool, según tu BD

// Creamos una función pública que devuelve el sub-router armado
pub fn router() -> Router<PgPool> {
    Router::new()
        .route("/registro", post(handlers::registrar_usuario))
} */
