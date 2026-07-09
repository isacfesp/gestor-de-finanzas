// =====================================================================
// consulta.rs — Lectura de audit_log con alcance de workspace. Aquí no
// se escribe nunca: cada módulo llama a `auditoria::registrar()` desde
// sus propios handlers, esto solo lee lo que ya quedó guardado.
// =====================================================================

use axum::{
    Json,
    extract::{Path, Query, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::movimientos::models::{Movimiento, PaginacionMovimientos};

/// GET /workspaces/:workspace_id/movimientos?limite=&desplazamiento=
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Query(pag): Query<PaginacionMovimientos>,
) -> Result<Json<Vec<Movimiento>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let limite = pag.limite.unwrap_or(50).clamp(1, 200);
    let desplazamiento = pag.desplazamiento.unwrap_or(0).max(0);

    let filas = sqlx::query_as!(
        Movimiento,
        r#"SELECT a.id, COALESCE(u.name, 'Sistema') AS "actor_name!",
                  a.action, a.detail, a.created_at
           FROM audit_log a
           LEFT JOIN users u ON u.id = a.user_id
           WHERE a.workspace_id = $1
           ORDER BY a.created_at DESC
           LIMIT $2 OFFSET $3"#,
        workspace_id,
        limite,
        desplazamiento
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}
