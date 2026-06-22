// =====================================================================
// handlers.rs — Funciones que responden a las peticiones HTTP
// Cada función aquí es un "handler": Axum llama a estas funciones automáticamente
// cuando llega una petición a la ruta correspondiente.
// =====================================================================

// De la librería "axum" (nuestro framework web):
// - State: extractor que le pide a Axum el estado compartido (el pool de BD)
// - StatusCode: constantes con los códigos HTTP (200, 201, 409, 500, etc.)
// - Json: wrapper que convierte structs a JSON y viceversa
use axum::{Json, extract::State, http::StatusCode};
// De la librería "serde_json":
// - json!: macro que construye un valor JSON directamente en el código.
use serde_json::json;
// De la librería "sqlx":
// - PgPool: el tipo del pool de conexiones a PostgreSQL.
use sqlx::PgPool;
//  (crate:: = raíz de nuestro proyecto):
// - RegistroDatos: la struct que representa el body del request de registro
// - User: la struct que representa una fila de la tabla users en la BD
use crate::users::models::{RegistroDatos, User};

// =====================================================================
// HANDLER: registrar_usuario
// Ruta: POST /usuarios/registro
// =====================================================================

// PARÁMETROS — Axum los inyecta automáticamente leyendo su tipo:
// State(pool): State<PgPool>
//   Axum ve que pedimos "State<PgPool>" y busca en el estado compartido
//   del Router (el que adjuntamos con .with_state(pool) en main.rs).
//   "State(pool)" es desestructuración: en lugar de recibir State(pool)
//   y luego escribir pool.0, Rust nos deja extraer el valor interior
//   directamente en la firma. "pool" ahora es de tipo PgPool.
//
// Json(datos): Json<RegistroDatos>
//   Axum lee el body del request HTTP, lo interpreta como JSON, y lo
//   convierte a nuestra struct RegistroDatos (gracias al derive Deserialize).
//   "Json(datos)" es la misma desestructuración — "datos" es RegistroDatos.
//   Si el JSON está malformado o le falta un campo, Axum devuelve 400
//   automáticamente antes de llegar a nuestro código.
//
// TIPO DE RETORNO: (StatusCode, Json<serde_json::Value>)
//   Axum acepta tuplas como respuesta. El primer elemento es el código HTTP,
//   el segundo es el body. serde_json::Value es un JSON genérico — nos
//   permite devolver cualquier forma de JSON sin necesitar una struct fija,
//   lo que es cómodo para respuestas de error que varían.
//
pub async fn registrar_usuario(
    State(pool): State<PgPool>,
    Json(datos): Json<RegistroDatos>,
) -> (StatusCode, Json<serde_json::Value>) {
    // bcrypt::hash devuelve un Result<String, Error>. .unwrap() lo abre
    // y si falla cierra el programa
    let hash = bcrypt::hash(&datos.password, bcrypt::DEFAULT_COST).unwrap();

    // sqlx::query_as! es un macro que hace tres cosas:
    //   1. Verifica que el SQL sea válido contra la BD real en tiempo de compilación
    //   2. Ejecuta la query con los parámetros ($1, $2, $3 = posiciones)
    //   3. Mapea el resultado directamente a nuestra struct "User"
    //
    // RETURNING id, name, email, password_hash, created_at
    //   Le pedimos a Postgres que nos devuelva la fila recién insertada.
    //   Así obtenemos el "id" y "created_at" que generó la BD automáticamente
    //   (gen_random_uuid() y now() en la migración).
    //
    // .fetch_one(&pool) ejecuta la query y espera exactamente 1 fila.
    // .await pausa esta función hasta que la BD responda (sin bloquear el servidor).
    //
    // El resultado es un Result<User, sqlx::Error> — puede ser Ok(usuario)
    // o Err(algo_salió_mal). Lo guardamos en "resultado" para revisarlo después.
    let resultado = sqlx::query_as!(
        User,
        "INSERT INTO users (name, email, password_hash)
         VALUES ($1, $2, $3)
         RETURNING id, name, email, password_hash, created_at",
        datos.name,
        datos.email,
        hash
    )
    .fetch_one(&pool)
    .await;

    let usuario = match resultado {
        Ok(u) => u,
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("users_email_unique") => {
            return (
                StatusCode::CONFLICT,
                Json(json!({"error": "El email ya está registrado"})),
            );
        }
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Error interno del servidor"})),
            );
        }
    };

    // -----------------------------------------------------------------
    // Crear el workspace personal del usuario
    // -----------------------------------------------------------------
    // Cada usuario nuevo recibe un workspace propio automáticamente.
    // .fetch_one devuelve directamente el UUID — Postgres lo genera con
    // gen_random_uuid() gracias al RETURNING id.
    let workspace_id: uuid::Uuid = sqlx::query_scalar!(
        "INSERT INTO workspaces (name, owner_id) VALUES ($1, $2) RETURNING id",
        format!("Workspace de {}", usuario.name),
        usuario.id
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // -----------------------------------------------------------------
    //  Agregar al usuario como admin de su workspace
    // -----------------------------------------------------------------
    // workspace_members es la tabla que relaciona usuarios con workspaces.
    // El creador del workspace siempre entra como 'admin'.
    // Usamos query! porque este INSERT no devuelve
    // nada — solo ejecuta. Por eso usamos .execute() en lugar de .fetch_one().
    sqlx::query!(
        "INSERT INTO workspace_members (workspace_id, user_id, role)
         VALUES ($1, $2, 'admin')",
        workspace_id,
        usuario.id
    )
    .execute(&pool)
    .await
    .unwrap();

    // -----------------------------------------------------------------
    // Devolver la respuesta exitosa
    // -----------------------------------------------------------------
    // Respondemos 201 CREATED con los datos del usuario recién creado.
    // json!() construye el objeto JSON directamente. Axum lo convierte
    // automáticamente al body de la respuesta HTTP.
    (
        StatusCode::CREATED,
        Json(json!({
            "id": usuario.id,
            "name": usuario.name,
            "email": usuario.email,
            "created_at": usuario.created_at,
        })),
    )
}
