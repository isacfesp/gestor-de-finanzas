// =====================================================================
// resumen.rs — Agregado de ahorro por inversiones para el Dashboard:
// capital invertido y rendimiento bruto/ISR/neto acumulado de todas
// las inversiones activas (según investment_accruals).
// =====================================================================

use axum::{
    Json,
    extract::{Path, State},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::autorizacion::{RolWorkspace, verificar_membresia};
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;
use crate::investments::models::ResumenAhorroInversiones;

/// GET /workspaces/:workspace_id/inversiones/resumen
///
/// Un `member` solo ve el agregado de sus propias inversiones; un
/// `admin`/dev ve el de todo el workspace (supervisión).
pub async fn obtener(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<ResumenAhorroInversiones>, AppError> {
    let rol = verificar_membresia(&pool, &usuario, workspace_id).await?;
    let solo_propias = matches!(rol, RolWorkspace::Member).then_some(usuario.id);

    // Los accruals se agregan por inversión ANTES de unir con
    // investments: si se uniera directo (LEFT JOIN sin agregar antes),
    // cada fila de accrual repetiría la fila de la inversión y
    // SUM(i.principal) contaría el capital una vez por cada día ya
    // calculado, no una vez por inversión.
    let fila = sqlx::query!(
        r#"WITH accruals AS (
               SELECT investment_id,
                      SUM(gross_yield) AS gross_yield,
                      SUM(isr_amount) AS isr_amount,
                      SUM(net_yield) AS net_yield
               FROM investment_accruals
               GROUP BY investment_id
           )
           SELECT
               COALESCE(SUM(i.principal), 0) AS "principal_invertido!",
               COALESCE(SUM(a.gross_yield), 0) AS "gross_yield_acumulado!",
               COALESCE(SUM(a.isr_amount), 0) AS "isr_acumulado!",
               COALESCE(SUM(a.net_yield), 0) AS "net_yield_acumulado!"
           FROM investments i
           LEFT JOIN accruals a ON a.investment_id = i.id
           WHERE i.workspace_id = $1 AND i.is_active = true
             AND ($2::uuid IS NULL OR i.owner_id = $2)"#,
        workspace_id,
        solo_propias
    )
    .fetch_one(&pool)
    .await?;

    Ok(Json(ResumenAhorroInversiones {
        principal_invertido: fila.principal_invertido,
        gross_yield_acumulado: fila.gross_yield_acumulado,
        isr_acumulado: fila.isr_acumulado,
        net_yield_acumulado: fila.net_yield_acumulado,
    }))
}
