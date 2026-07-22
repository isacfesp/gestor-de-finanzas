// =====================================================================
// correo/mod.rs — Envío de correos transaccionales vía la API de Resend.
//
// Se usa para: link de invitación a un workspace, link de recuperación
// de contraseña y alertas de `reminders`. El envío es siempre "best
// effort" desde quien llama: un fallo se loggea pero nunca debe tumbar
// la operación HTTP ni el ciclo de fondo que lo dispara.
// =====================================================================

mod plantillas;

pub use plantillas::{plantilla_alerta, plantilla_invitacion, plantilla_recuperacion};

use reqwest::Client;
use serde_json::json;
use std::sync::OnceLock;

/// Cliente HTTP reutilizado en cada envío: crear uno nuevo por llamada
/// desperdiciaría el pool de conexiones que reqwest mantiene internamente.
fn cliente() -> &'static Client {
    static CLIENTE: OnceLock<Client> = OnceLock::new();
    CLIENTE.get_or_init(Client::new)
}

/// Primer origen listado en FRONTEND_ORIGIN (puede traer varios
/// separados por coma, ver `construir_cors` en `main.rs`). Se usa para
/// armar los links que van dentro de los correos.
pub fn frontend_origin() -> String {
    let origenes_por_defecto = "http://localhost:8080";
    std::env::var("FRONTEND_ORIGIN")
        .unwrap_or_else(|_| origenes_por_defecto.to_string())
        .split(',')
        .next()
        .unwrap_or(origenes_por_defecto)
        .trim()
        .to_string()
}

/// Envía un correo HTML a un destinatario vía la API de Resend.
///
/// Devuelve `Err` con un detalle apto para loggear (nunca se lo
/// muestra al cliente HTTP) si faltan las variables de entorno, si
/// Resend responde un error o si falla la conexión.
pub async fn enviar(destinatario: &str, asunto: &str, html: &str) -> Result<(), String> {
    let api_key =
        std::env::var("RESEND_API_KEY").map_err(|_| "RESEND_API_KEY no configurada".to_string())?;
    let remitente =
        std::env::var("RESEND_FROM").map_err(|_| "RESEND_FROM no configurada".to_string())?;

    let respuesta = cliente()
        .post("https://api.resend.com/emails")
        .bearer_auth(api_key)
        .json(&json!({
            "from": remitente,
            "to": [destinatario],
            "subject": asunto,
            "html": html,
        }))
        .send()
        .await
        .map_err(|e| format!("error de red hacia Resend: {e}"))?;

    if !respuesta.status().is_success() {
        let codigo = respuesta.status();
        let cuerpo = respuesta.text().await.unwrap_or_default();
        return Err(format!("Resend respondió {codigo}: {cuerpo}"));
    }
    Ok(())
}
