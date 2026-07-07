// =====================================================================
// categorias.rs — Gestión de categorías de transacción.
//
// Una categoría puede ser global (workspace_id NULL, visible desde
// cualquier workspace) o personalizada de un workspace. Por ahora solo
// se pueden crear categorías personalizadas desde la API; las globales
// se reservan para una futura carga administrativa.
// =====================================================================

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::accounting::models::{Categoria, CrearCategoriaDatos};
use crate::auth::autorizacion::verificar_membresia;
use crate::auth::extractores::UsuarioAutenticado;
use crate::errores::AppError;

/// Valida que el tipo sea uno de los que acepta el CHECK de la tabla.
fn validar_tipo(tipo: &str) -> Result<(), AppError> {
    if tipo == "income" || tipo == "expense" {
        Ok(())
    } else {
        Err(AppError::NoProcesable(
            "El tipo debe ser 'income' o 'expense'".to_string(),
        ))
    }
}

/// Confirma que `category_id` es visible desde `workspace_id` (global o
/// propia) y que su tipo coincide con `tipo_esperado`. La usan
/// transacciones, suscripciones y presupuestos antes de referenciar una
/// categoría, para no aceptar una categoría ajena ni de tipo incorrecto.
pub(crate) async fn validar_categoria(
    pool: &PgPool,
    category_id: Uuid,
    workspace_id: Uuid,
    tipo_esperado: &str,
) -> Result<(), AppError> {
    let tipo = sqlx::query_scalar!(
        r#"SELECT type FROM categories
           WHERE id = $1 AND (workspace_id IS NULL OR workspace_id = $2)"#,
        category_id,
        workspace_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NoEncontrado("Categoría no encontrada".to_string()))?;

    if tipo != tipo_esperado {
        return Err(AppError::NoProcesable(format!(
            "La categoría es de tipo '{tipo}', se esperaba '{tipo_esperado}'"
        )));
    }
    Ok(())
}

/// GET /workspaces/:workspace_id/categorias — globales + propias del workspace.
pub async fn listar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
) -> Result<Json<Vec<Categoria>>, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let filas = sqlx::query_as!(
        Categoria,
        r#"SELECT id, workspace_id, name, type AS "tipo"
           FROM categories
           WHERE workspace_id IS NULL OR workspace_id = $1
           ORDER BY name"#,
        workspace_id
    )
    .fetch_all(&pool)
    .await?;

    Ok(Json(filas))
}

/// POST /workspaces/:workspace_id/categorias — crea una categoría propia.
pub async fn crear(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path(workspace_id): Path<Uuid>,
    Json(datos): Json<CrearCategoriaDatos>,
) -> Result<(StatusCode, Json<Categoria>), AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;
    validar_tipo(&datos.tipo)?;

    if datos.name.trim().is_empty() {
        return Err(AppError::NoProcesable(
            "El nombre no puede estar vacío".to_string(),
        ));
    }

    let resultado = sqlx::query_as!(
        Categoria,
        r#"INSERT INTO categories (workspace_id, name, type)
           VALUES ($1, $2, $3)
           RETURNING id, workspace_id, name, type AS "tipo""#,
        workspace_id,
        datos.name.trim(),
        datos.tipo
    )
    .fetch_one(&pool)
    .await;

    match resultado {
        Ok(categoria) => Ok((StatusCode::CREATED, Json(categoria))),
        Err(sqlx::Error::Database(e))
            if e.constraint() == Some("categories_workspace_name_unique") =>
        {
            Err(AppError::Conflicto(
                "Ya existe una categoría con ese nombre en este workspace".to_string(),
            ))
        }
        Err(e) => Err(e.into()),
    }
}

/// DELETE /workspaces/:workspace_id/categorias/:id — solo categorías propias.
pub async fn eliminar(
    State(pool): State<PgPool>,
    usuario: UsuarioAutenticado,
    Path((workspace_id, id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, AppError> {
    verificar_membresia(&pool, &usuario, workspace_id).await?;

    let resultado = sqlx::query!(
        "DELETE FROM categories WHERE id = $1 AND workspace_id = $2",
        id,
        workspace_id
    )
    .execute(&pool)
    .await;

    match resultado {
        Ok(r) if r.rows_affected() == 0 => Err(AppError::NoEncontrado(
            "Categoría no encontrada".to_string(),
        )),
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        // Violación de FK: la categoría está referenciada por transacciones,
        // suscripciones, presupuestos o previstos.
        Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23503") => Err(
            AppError::Conflicto("La categoría está en uso, no se puede eliminar".to_string()),
        ),
        Err(e) => Err(e.into()),
    }
}
