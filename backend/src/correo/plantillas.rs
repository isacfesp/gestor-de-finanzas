// =====================================================================
// correo/plantillas.rs — Cuerpos HTML de los correos transaccionales.
//
// Solo 3 correos en todo el sistema: no amerita un motor de plantillas,
// con format! alcanza. Todo el CSS va inline (nada de <style> ni
// clases): los clientes de correo ignoran las hojas de estilo externas
// y muchos también recortan el <style> del <head>.
// =====================================================================

use super::frontend_origin;

/// Mismos colores que `--card`/`--text`/`--accent` del tema oscuro de
/// `frontend/styles/tailwind.css`, para que el correo se sienta del
/// mismo sistema de diseño ("Geck") que la app.
const COLOR_FONDO: &str = "#f4f6fb";
const COLOR_TARJETA: &str = "#0d1836";
const COLOR_TEXTO: &str = "#eaf0ff";
const COLOR_TEXTO_SUAVE: &str = "#9aa4c4";
const COLOR_ACCENT: &str = "#06b6d4";
const COLOR_ACCENT_2: &str = "#22d3ee";

/// URL pública del ícono PWA (192x192), reutilizado como logo del
/// correo — es PNG, a diferencia de `gecko.svg`, que muchos clientes de
/// correo (Outlook clásico entre otros) no renderizan.
fn logo_url() -> String {
    format!("{}/assets/icon-192.png", frontend_origin())
}

/// Envuelve el contenido de un correo en el mismo esqueleto: logo +
/// wordmark arriba, tarjeta oscura con el mensaje, pie de página.
/// `table role="presentation"` es la forma estándar de maquetar correos
/// sin depender de flexbox/grid, que Outlook no soporta.
fn envolver_correo(cuerpo_html: &str) -> String {
    let logo = logo_url();
    format!(
        r#"<!doctype html>
<html>
<body style="margin:0;padding:32px 16px;background:{COLOR_FONDO};font-family:-apple-system,'Segoe UI',Roboto,Helvetica,Arial,sans-serif;">
<table role="presentation" width="100%" style="max-width:480px;margin:0 auto;border-collapse:collapse;">
<tr>
<td style="padding-bottom:20px;text-align:center;">
<img src="{logo}" width="40" height="40" alt="Geck" style="display:inline-block;vertical-align:middle;border-radius:8px;" />
<span style="font-size:20px;font-weight:700;color:#0c1b3a;vertical-align:middle;margin-left:8px;">geck</span>
</td>
</tr>
<tr>
<td style="background:{COLOR_TARJETA};border-radius:16px;padding:32px;color:{COLOR_TEXTO};font-size:15px;line-height:1.6;">
{cuerpo_html}
</td>
</tr>
<tr>
<td style="padding-top:20px;text-align:center;font-size:12px;color:{COLOR_TEXTO_SUAVE};">
Gestor de Finanzas &middot; correo automático, no respondas a este mensaje.
</td>
</tr>
</table>
</body>
</html>"#
    )
}

/// Botón de acción (CTA): fondo con el color de acento, texto oscuro
/// para contraste — mismo par que usa `.btn-primary` en la app.
fn boton(texto: &str, link: &str) -> String {
    format!(
        r#"<p style="text-align:center;margin:28px 0;">
<a href="{link}" style="background:{COLOR_ACCENT};color:#04202a;font-weight:700;text-decoration:none;padding:12px 28px;border-radius:10px;display:inline-block;">{texto}</a>
</p>"#
    )
}

/// Párrafo de respaldo con el link plano, para cuando el botón no se
/// pueda clickear (algunos clientes de correo lo bloquean por defecto).
fn link_de_respaldo(link: &str) -> String {
    format!(
        r#"<p style="margin:12px 0 0;color:{COLOR_TEXTO_SUAVE};font-size:13px;">Si el botón no funciona, copia y pega este link:<br/>
<a href="{link}" style="color:{COLOR_ACCENT_2};word-break:break-all;">{link}</a></p>"#
    )
}

/// Correo de invitación a un workspace: `link` ya incluye el token
/// (`{frontend_origin}/invitaciones/aceptar?token=...`).
pub fn plantilla_invitacion(link: &str) -> String {
    let cuerpo = format!(
        r#"<p style="margin:0;">Te invitaron a un workspace en <strong>Gestor de Finanzas</strong>.</p>
{boton}
<p style="margin:0;color:{COLOR_TEXTO_SUAVE};font-size:13px;">Este link expira en 72 horas y solo funciona una vez.</p>
{respaldo}"#,
        boton = boton("Aceptar invitación", link),
        respaldo = link_de_respaldo(link),
    );
    envolver_correo(&cuerpo)
}

/// Correo de recuperación de contraseña: `link` ya incluye el token
/// (`{frontend_origin}/recuperar-password?token=...`).
pub fn plantilla_recuperacion(link: &str) -> String {
    let cuerpo = format!(
        r#"<p style="margin:0;">Pediste recuperar tu contraseña en <strong>Gestor de Finanzas</strong>.</p>
{boton}
<p style="margin:0;color:{COLOR_TEXTO_SUAVE};font-size:13px;">Este link expira en 1 hora y solo funciona una vez. Si no fuiste tú, ignora este correo.</p>
{respaldo}"#,
        boton = boton("Elegir contraseña nueva", link),
        respaldo = link_de_respaldo(link),
    );
    envolver_correo(&cuerpo)
}

/// Correo de una alerta del módulo `reminders` (suscripción por vencer,
/// presupuesto, tarjeta de crédito). `titulo` y `body` son los mismos
/// textos que ya se guardan en `notifications`.
pub fn plantilla_alerta(titulo: &str, body: &str) -> String {
    let cuerpo = format!(
        r#"<p style="margin:0 0 6px;color:{COLOR_ACCENT_2};font-weight:700;font-size:16px;">{titulo}</p>
<p style="margin:0;">{body}</p>"#
    );
    envolver_correo(&cuerpo)
}
