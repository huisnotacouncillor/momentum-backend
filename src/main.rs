use axum::{Router, Server, middleware};
use diesel::PgConnection;
use diesel::r2d2::{self, ConnectionManager as DbConnectionManager};
use rust_backend::{AppState, db::DbPool, websocket};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();
    let config = rust_backend::config::Config::from_env();

    // Initialize database
    let manager = DbConnectionManager::<PgConnection>::new(&config.db_url);
    let db: DbPool = r2d2::Pool::builder()
        .build(manager)
        .expect("Failed to create database connection pool");

    // Initialize Redis
    let redis = redis::Client::open(config.redis_url).expect("Failed to create Redis client");

    // Application state
    let state = std::sync::Arc::new(AppState { db, redis });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create WebSocket state and start cleanup task
    let ws_state = websocket::create_websocket_state(std::sync::Arc::new(state.db.clone()));
    let ws_manager = ws_state.ws_manager.clone();

    // Start WebSocket cleanup task
    tokio::spawn(async move {
        websocket::start_connection_cleanup_task(ws_manager).await;
    });

    // Build router
    let app = Router::new()
        .merge(rust_backend::routes::create_router(state.db.clone()))
        .layer(cors)
        .layer(middleware::from_fn(
            rust_backend::middleware::logger::logger,
        ));

    // Start server
    let addr = "127.0.0.1:8000".parse().unwrap();
    println!("Server running at http://{}", addr);
    println!("WebSocket endpoint available at ws://{}/ws", addr);
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
