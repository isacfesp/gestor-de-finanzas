//! Helpers de bajo nivel para hablar con la API del backend.
//!
//! Cada módulo de dominio (auth, cuentas, metas...) construye sus
//! propias funciones tipadas sobre estos cuatro verbos en vez de
//! llamar a `gloo_net` directamente, así el manejo de errores y el
//! header de autenticación quedan en un solo lugar.

use gloo_net::http::{Request, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::error::{ApiError, ErrorCuerpo};

/// Decodifica una respuesta como JSON si fue exitosa (2xx), o como
/// `ApiError::Servidor` leyendo el `{"error": "..."}` si no.
async fn decodificar<T: DeserializeOwned>(respuesta: Response) -> Result<T, ApiError> {
    if respuesta.ok() {
        respuesta
            .json::<T>()
            .await
            .map_err(|e| ApiError::Red(e.to_string()))
    } else {
        let mensaje = respuesta
            .json::<ErrorCuerpo>()
            .await
            .map(|cuerpo| cuerpo.error)
            .unwrap_or_else(|_| format!("Error {}", respuesta.status()));
        Err(ApiError::Servidor(mensaje))
    }
}

fn header_autorizacion(token: &str) -> String {
    format!("Bearer {token}")
}

/// GET autenticado.
pub async fn get<T: DeserializeOwned>(path: &str, token: &str) -> Result<T, ApiError> {
    let respuesta = Request::get(path)
        .header("Authorization", &header_autorizacion(token))
        .send()
        .await
        .map_err(|e| ApiError::Red(e.to_string()))?;
    decodificar(respuesta).await
}

/// POST sin autenticación (login).
pub async fn post_publico<B: Serialize, T: DeserializeOwned>(
    path: &str,
    cuerpo: &B,
) -> Result<T, ApiError> {
    let respuesta = Request::post(path)
        .json(cuerpo)
        .map_err(|e| ApiError::Red(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Red(e.to_string()))?;
    decodificar(respuesta).await
}

// post/put/delete aún no los consume ningún módulo (solo auth usa GET
// y los dos POST de abajo), pero son la base con la que accounts,
// goals, investments, etc. van a crear/editar/borrar sus recursos.
#[allow(dead_code)]
/// POST autenticado.
pub async fn post<B: Serialize, T: DeserializeOwned>(
    path: &str,
    cuerpo: &B,
    token: &str,
) -> Result<T, ApiError> {
    let respuesta = Request::post(path)
        .header("Authorization", &header_autorizacion(token))
        .json(cuerpo)
        .map_err(|e| ApiError::Red(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Red(e.to_string()))?;
    decodificar(respuesta).await
}

#[allow(dead_code)]
/// PUT autenticado.
pub async fn put<B: Serialize, T: DeserializeOwned>(
    path: &str,
    cuerpo: &B,
    token: &str,
) -> Result<T, ApiError> {
    let respuesta = Request::put(path)
        .header("Authorization", &header_autorizacion(token))
        .json(cuerpo)
        .map_err(|e| ApiError::Red(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Red(e.to_string()))?;
    decodificar(respuesta).await
}

/// Confirma éxito (2xx) sin intentar decodificar el cuerpo como JSON.
/// Para endpoints que responden 204 No Content, donde no hay nada que
/// parsear.
async fn confirmar(respuesta: Response) -> Result<(), ApiError> {
    if respuesta.ok() {
        Ok(())
    } else {
        let mensaje = respuesta
            .json::<ErrorCuerpo>()
            .await
            .map(|cuerpo| cuerpo.error)
            .unwrap_or_else(|_| format!("Error {}", respuesta.status()));
        Err(ApiError::Servidor(mensaje))
    }
}

#[allow(dead_code)]
/// DELETE autenticado. Los endpoints de borrado devuelven 204 sin cuerpo.
pub async fn delete(path: &str, token: &str) -> Result<(), ApiError> {
    let respuesta = Request::delete(path)
        .header("Authorization", &header_autorizacion(token))
        .send()
        .await
        .map_err(|e| ApiError::Red(e.to_string()))?;
    confirmar(respuesta).await
}

/// POST autenticado cuya respuesta no trae cuerpo (204), como logout.
pub async fn post_sin_respuesta<B: Serialize>(
    path: &str,
    cuerpo: &B,
    token: &str,
) -> Result<(), ApiError> {
    let respuesta = Request::post(path)
        .header("Authorization", &header_autorizacion(token))
        .json(cuerpo)
        .map_err(|e| ApiError::Red(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiError::Red(e.to_string()))?;
    confirmar(respuesta).await
}
