use axum::{Router, Server, middleware::from_fn};
use momentum_api::middleware::{performance_monitoring_middleware, request_tracking_middleware};
use momentum_api::{AppState, websocket, AppConfig};
use momentum_core::db as core_db;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use momentum_core::config::Config;
use momentum_core::utils::AssetUrlHelper;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration from core
    let core_config = Config::from_env()?;
    let config = AppConfig::from_core_config(core_config.clone());

    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("Starting server with config: {:?}", config);

    // Initialize database
    let db_pool = core_db::create_pool(&core_config.database())?;

    // Test database connection
    core_db::pool_health_check(&db_pool).await?;

    // Initialize Redis
    let redis = redis::Client::open(config.redis_url.clone())?;

    // Asset helper
    let asset_helper = AssetUrlHelper::new(&momentum_core::utils::AssetConfig {
        base_url: core_config.assets_url.clone(),
    });

    // Application state
    let state = Arc::new(AppState::new(db_pool, redis, asset_helper));

    // CORS configuration
    let cors = if config.cors_origins.contains(&"*".to_string()) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let origins: Result<Vec<_>, _> = config
            .cors_origins
            .iter()
            .map(|origin| origin.parse())
            .collect();

        CorsLayer::new()
            .allow_origin(origins?)
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Create WebSocket state and start cleanup task
    let ws_state = websocket::create_websocket_state(Arc::new(state.db.clone()), &core_config);
    let ws_manager = ws_state.ws_manager.clone();

    // Start WebSocket cleanup task
    tokio::spawn(async move {
        websocket::start_connection_cleanup_task(ws_manager).await;
    });

    // Create the auth routes that don't need authentication
    let auth_routes = Router::new()
        .route(
            "/auth/register",
            axum::routing::post(momentum_api::routes::auth::register),
        )
        .route(
            "/auth/login",
            axum::routing::post(momentum_api::routes::auth::login),
        )
        .with_state(state.clone());

    // Build router - apply auth middleware only to routes that need it
    let protected_routes = momentum_api::routes::create_router(state.clone()).layer(
        axum::middleware::from_fn_with_state(
            Arc::new(state.db.clone()),
            momentum_api::middleware::auth::auth_middleware,
        ),
    );

    let app = Router::new()
        .merge(auth_routes)
        .merge(protected_routes)
        .merge(websocket::create_websocket_routes().with_state(ws_state))
        .layer(cors)
        .layer(from_fn(request_tracking_middleware))
        .layer(from_fn(performance_monitoring_middleware))
        .layer(from_fn(momentum_api::middleware::logger::logger));

    // Start server
    let addr = config.server_address;
    tracing::info!("Server running at http://{}", addr);
    tracing::info!("WebSocket endpoint available at ws://{}/ws", addr);

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}