use axum::{Router, routing::get};
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;

//prueba
async fn responder_salud() -> &'static str {
    "El servidor esta vivo"
}

#[tokio::main]

async fn main() {
    // Cargar el archivo .env a la memoria del proceso
    dotenvy::dotenv().ok();
    //extraer url del .env
    let url_db = std::env::var("DATABASE_URL")
        .expect("CRÍTICO: NO SE OBTUVO LA VARIABLE O ESTA MAL CONFIGURADA");
    //crear el pool de conexiones asincrinicas
    let pool = PgPool::connect(&url_db)
        .await
        .expect("CRÍTICO: No se pudo establecer el pool de conexiones con PostgreSQL");
    //sincronizar tablas de sql
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("CRÍTICO: Error al ejecutar las migraciones en la base de datos");

    //construir router
    let aplication = Router::new()
        .route("/salud", get(responder_salud))
        .with_state(pool); // <-- Aquí el Router se transforma en un Router<PgPool>

    // Abrir el puerto y bucle infinito del servidor
    let direccion = SocketAddr::from(([0, 0, 0, 0], 3000));
    let puerta_de_red = TcpListener::bind(direccion).await.unwrap();

    println!("SERVIDOR AXUM CORRIENDO EN http://localhost:3000");
    // Arrancamos el servidor infinito.
    // Le pasamos la puerta de red y el Router.
    // Usamos .await para que el programa se quede aquí ejecutándose para siempre.
    axum::serve(puerta_de_red, aplication).await.unwrap();
}
