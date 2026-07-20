mod accounting;
mod accounts;
mod admin;
mod analytics;
mod auditoria;
mod auth;
mod errores;
mod goals;
mod investments;
mod movimientos;
mod planned_transactions;
mod reminders;
mod tags;
mod users;

use axum::{
    Router,
    http::{HeaderValue, Method, header},
    routing::get,
};
use sqlx::PgPool;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

/// Endpoint de diagnóstico: responde texto plano para comprobar
/// que el servidor está vivo (GET /salud).
async fn responder_salud() -> &'static str {
    "El servidor esta vivo"
}

/// Lee el puerto del servidor desde la variable de entorno PORT.
///
/// Devuelve 3000 si PORT no está definida. Falla con un error claro
/// si PORT existe pero no es un número válido (mejor avisar que
/// arrancar en un puerto que no era el esperado).
fn leer_puerto() -> Result<u16, Box<dyn Error>> {
    match std::env::var("PORT") {
        Ok(valor) => valor.parse::<u16>().map_err(|_| {
            format!(
                "PORT tiene un valor inválido: '{valor}' (se esperaba un número entre 1 y 65535)"
            )
            .into()
        }),
        Err(_) => Ok(3000),
    }
}

/// Arma la política CORS: solo los orígenes del frontend pueden llamar
/// a la API desde el navegador.
///
/// Lee FRONTEND_ORIGIN del .env (lista separada por comas). Si no está
/// definida, cae en los puertos por defecto de `trunk serve` en
/// desarrollo local. No se permiten credenciales de cookies porque la
/// autenticación viaja en el header Authorization, no en cookies.
fn construir_cors() -> CorsLayer {
    let origenes_por_defecto = "http://localhost:8080,http://127.0.0.1:8080";
    let origenes =
        std::env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| origenes_por_defecto.to_string());

    let valores: Vec<HeaderValue> = origenes
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .filter_map(|origen| origen.parse::<HeaderValue>().ok())
        .collect();

    CorsLayer::new()
        .allow_origin(valores)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([header::CONTENT_TYPE, header::AUTHORIZATION])
}

/// Conecta con PostgreSQL y deja la base de datos lista para usarse.
///
/// Lee DATABASE_URL del entorno, crea el pool de conexiones asíncronas
/// y ejecuta las migraciones pendientes de backend/migrations/.
/// Falla si la variable no existe, si la base no responde o si una
/// migración tiene errores.
async fn preparar_base_de_datos() -> Result<PgPool, Box<dyn Error>> {
    let url_db = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL no está definida o está mal configurada en el .env")?;

    let pool = PgPool::connect(&url_db)
        .await
        .map_err(|e| format!("No se pudo conectar con PostgreSQL: {e}"))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| format!("Error al ejecutar las migraciones: {e}"))?;

    Ok(pool)
}

/// Garantiza que exista al menos un usuario con rol dev.
///
/// Primera vez que arranca el sistema (base vacía): crea el dev con
/// DEV_EMAIL y DEV_PASSWORD del .env. En arranques posteriores ya
/// existe un dev y esta función no hace nada — las variables pueden
/// incluso borrarse del .env después del primer arranque.
async fn bootstrap_dev(pool: &PgPool) -> Result<(), Box<dyn Error>> {
    let hay_dev = sqlx::query_scalar!("SELECT EXISTS(SELECT 1 FROM users WHERE role = 'dev')")
        .fetch_one(pool)
        .await?
        .unwrap_or(false);

    if hay_dev {
        return Ok(());
    }

    let email = std::env::var("DEV_EMAIL")
        .map_err(|_| "No existe ningún dev y falta DEV_EMAIL en el .env para crearlo")?;
    let password = std::env::var("DEV_PASSWORD")
        .map_err(|_| "No existe ningún dev y falta DEV_PASSWORD en el .env para crearlo")?;

    let mut tx = pool.begin().await?;
    let dev = users::servicio::crear_usuario(&mut tx, "Dev", &email, &password, "dev")
        .await
        .map_err(|e| format!("No se pudo crear el usuario dev inicial: {e:?}"))?;
    tx.commit().await?;

    auditoria::registrar(
        pool,
        None,
        Some(dev.id),
        auditoria::acciones::BOOTSTRAP_DEV,
        serde_json::json!({"email": dev.email}),
    )
    .await;

    println!("Usuario dev inicial creado ({})", dev.email);
    Ok(())
}

/// Construye el router de la aplicación con todas sus rutas.
///
/// El pool se registra como estado compartido: cada handler puede
/// pedirlo con el extractor State<PgPool>.
fn construir_router(pool: PgPool) -> Router {
    // Varios módulos se anidan bajo el mismo prefijo /workspaces/:workspace_id:
    // axum combina sus árboles de rutas siempre que no compartan una misma
    // ruta final y usen el mismo nombre de parámetro en cada posición.
    Router::new()
        .route("/salud", get(responder_salud))
        .nest("/auth", auth::router())
        .nest("/admin", admin::router())
        .nest("/workspaces/:workspace_id", accounting::router())
        .nest("/workspaces/:workspace_id", accounts::router())
        .nest("/workspaces/:workspace_id", planned_transactions::router())
        .nest("/workspaces/:workspace_id", tags::router())
        .nest("/workspaces/:workspace_id", goals::router())
        .nest("/workspaces/:workspace_id", investments::router())
        .nest("/workspaces/:workspace_id", analytics::router())
        .nest("/workspaces/:workspace_id", reminders::router())
        .nest("/workspaces/:workspace_id", movimientos::router())
        .layer(construir_cors())
        .with_state(pool)
}

/// Punto de entrada: carga el .env, prepara la base de datos y deja
/// el servidor Axum atendiendo peticiones indefinidamente.
///
/// Devuelve Result para que cualquier fallo de arranque termine el
/// programa con un mensaje claro en vez de un panic.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Carga el archivo .env al entorno del proceso. dotenvy busca el .env
    // en el directorio actual y en sus carpetas padre (por eso vive en la
    // raíz del repo y funciona igual con cargo run que dentro de Docker).
    dotenvy::dotenv().ok();

    let puerto = leer_puerto()?;
    let pool = preparar_base_de_datos().await?;
    bootstrap_dev(&pool).await?;

    // PgPool es un Arc por dentro: clonarlo es barato. Ambos motores
    // corren en el mismo proceso, no hay un binario worker separado
    // (ver docker-compose.yml). Van en ciclos independientes: un fallo
    // en el cálculo de accrual de inversiones no debe frenar las
    // notificaciones, ni viceversa.
    tokio::spawn(reminders::motor::ejecutar_ciclo_periodico(pool.clone()));
    tokio::spawn(investments::motor::ejecutar_ciclo_periodico(pool.clone()));

    let aplicacion = construir_router(pool);

    let direccion = SocketAddr::from(([0, 0, 0, 0], puerto));
    let puerta_de_red = TcpListener::bind(direccion)
        .await
        .map_err(|e| format!("No se pudo abrir el puerto {puerto}: {e}"))?;

    println!("SERVIDOR AXUM CORRIENDO EN http://localhost:{puerto}");
    // El servidor se queda aquí atendiendo peticiones para siempre;
    // solo retorna si ocurre un error irrecuperable.
    axum::serve(puerta_de_red, aplicacion)
        .await
        .map_err(|e| format!("El servidor terminó con un error: {e}"))?;

    Ok(())
}
