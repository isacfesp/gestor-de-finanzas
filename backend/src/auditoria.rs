// =====================================================================
// auditoria.rs — Bitácora de acciones sensibles (tabla audit_log).
//
// La auditoría es "best effort": si el INSERT falla, se loggea el
// problema pero NO se interrumpe la operación principal — un fallo
// al auditar no debe tirar un login válido.
// =====================================================================

use sqlx::PgPool;
use uuid::Uuid;

/// Acciones que se registran. Usar siempre estas constantes (y no
/// strings sueltos) para poder filtrar la bitácora sin errores de dedo.
pub mod acciones {
    pub const LOGIN_OK: &str = "login_ok";
    pub const LOGIN_FALLIDO: &str = "login_fallido";
    pub const LOGIN_BLOQUEADO: &str = "login_bloqueado";
    pub const LOGOUT: &str = "logout";
    pub const REFRESH_REUSO: &str = "refresh_reuso";
    pub const USUARIO_CREADO: &str = "usuario_creado";
    pub const WORKSPACE_CREADO: &str = "workspace_creado";
    pub const WORKSPACE_ELIMINADO: &str = "workspace_eliminado";
    pub const MIEMBRO_ASIGNADO: &str = "miembro_asignado";
    pub const MIEMBRO_ELIMINADO: &str = "miembro_eliminado";
    pub const INVITACION_CREADA: &str = "invitacion_creada";
    pub const INVITACION_ACEPTADA: &str = "invitacion_aceptada";
    pub const BOOTSTRAP_DEV: &str = "bootstrap_dev";
    pub const USUARIO_DESACTIVADO: &str = "usuario_desactivado";
    pub const USUARIO_REACTIVADO: &str = "usuario_reactivado";
    pub const RECUPERACION_SOLICITADA: &str = "recuperacion_solicitada";
    pub const PASSWORD_RESETEADO: &str = "password_reseteado";

    // accounts
    pub const CUENTA_CREADA: &str = "cuenta_creada";
    pub const CUENTA_EDITADA: &str = "cuenta_editada";
    pub const CUENTA_ELIMINADA: &str = "cuenta_eliminada";
    pub const TRANSFERENCIA_CREADA: &str = "transferencia_creada";

    // tags
    pub const ETIQUETA_CREADA: &str = "etiqueta_creada";
    pub const ETIQUETA_ELIMINADA: &str = "etiqueta_eliminada";
    pub const ETIQUETA_ASOCIADA: &str = "etiqueta_asociada";
    pub const ETIQUETA_DESASOCIADA: &str = "etiqueta_desasociada";

    // planned_transactions
    pub const PREVISTO_CREADO: &str = "previsto_creado";
    pub const PREVISTO_EDITADO: &str = "previsto_editado";
    pub const PREVISTO_PAGADO: &str = "previsto_pagado";
    pub const PREVISTO_ELIMINADO: &str = "previsto_eliminado";

    // accounting
    pub const CATEGORIA_CREADA: &str = "categoria_creada";
    pub const CATEGORIA_ELIMINADA: &str = "categoria_eliminada";
    pub const TRANSACCION_CREADA: &str = "transaccion_creada";
    pub const TRANSACCION_EDITADA: &str = "transaccion_editada";
    pub const TRANSACCION_ELIMINADA: &str = "transaccion_eliminada";
    pub const SUSCRIPCION_CREADA: &str = "suscripcion_creada";
    pub const SUSCRIPCION_EDITADA: &str = "suscripcion_editada";
    pub const SUSCRIPCION_COBRADA: &str = "suscripcion_cobrada";
    pub const SUSCRIPCION_COBRADA_AUTOMATICA: &str = "suscripcion_cobrada_automatica";
    pub const SUSCRIPCION_ELIMINADA: &str = "suscripcion_eliminada";
    pub const PRESUPUESTO_GUARDADO: &str = "presupuesto_guardado";
    pub const PRESUPUESTO_ELIMINADO: &str = "presupuesto_eliminado";

    // goals
    pub const META_CREADA: &str = "meta_creada";
    pub const META_EDITADA: &str = "meta_editada";
    pub const META_ELIMINADA: &str = "meta_eliminada";
    pub const APORTE_REGISTRADO: &str = "aporte_registrado";

    // investments
    pub const INVERSION_CREADA: &str = "inversion_creada";
    pub const INVERSION_EDITADA: &str = "inversion_editada";
    pub const INVERSION_ELIMINADA: &str = "inversion_eliminada";
    pub const RENDIMIENTO_REGISTRADO: &str = "rendimiento_registrado";
}

/// Registra una acción en la bitácora.
///
/// `workspace_id` es None para eventos globales, sin workspace asociado
/// (login, logout, alta de usuario, bootstrap). `user_id` es None cuando
/// no hay usuario identificado (p. ej. un intento de login con un email
/// inexistente). `detalle` es JSON libre con contexto útil — nunca
/// incluir contraseñas ni tokens.
pub async fn registrar(
    pool: &PgPool,
    workspace_id: Option<Uuid>,
    user_id: Option<Uuid>,
    accion: &str,
    detalle: serde_json::Value,
) {
    let resultado = sqlx::query!(
        "INSERT INTO audit_log (workspace_id, user_id, action, detail) VALUES ($1, $2, $3, $4)",
        workspace_id,
        user_id,
        accion,
        detalle
    )
    .execute(pool)
    .await;

    if let Err(e) = resultado {
        eprintln!("AVISO: no se pudo registrar auditoría '{accion}': {e}");
    }
}
