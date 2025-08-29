use axum::{Router, Server, middleware::from_fn};
use diesel::{PgConnection, r2d2::{self, ConnectionManager as DbConnectionManager}};
use rust_backend::{AppState, db::DbPool, websocket};
use tower_http::cors::{Any, CorsLayer};
use std::sync::Arc;
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
    let state = Arc::new(AppState { db, redis });

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Create WebSocket state and start cleanup task
    let ws_state = websocket::create_websocket_state(Arc::new(state.db.clone()));
    let ws_manager = ws_state.ws_manager.clone();

    // Start WebSocket cleanup task
    tokio::spawn(async move {
        websocket::start_connection_cleanup_task(ws_manager).await;
    });

    // Create the auth routes that don't need authentication
    let auth_routes = Router::new()
        .route("/auth/register", axum::routing::post(rust_backend::routes::auth::register))
        .route("/auth/login", axum::routing::post(rust_backend::routes::auth::login))
        .route("/auth/refresh", axum::routing::post(rust_backend::routes::auth::refresh_token))
        .with_state(Arc::new(state.db.clone()));

    // Build router - apply auth middleware only to routes that need it
    let protected_routes = rust_backend::routes::create_router(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(state.db.clone()),
            rust_backend::middleware::auth::auth_middleware,
        ));

    let app = Router::new()
        .merge(auth_routes)
        .merge(protected_routes)
        .merge(websocket::create_websocket_routes().with_state(ws_state))
        .layer(cors)
        .layer(from_fn(
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