use axum::{Router, middleware::from_fn};
use momentum_api::middleware::{performance_monitoring_middleware, request_tracking_middleware};
use momentum_api::{AppConfig, AppState, websocket};
use momentum_core::config::Config;
use momentum_core::db as core_db;
use momentum_core::utils::AssetUrlHelper;
use std::sync::Arc;
use tokio::signal;
use tower_http::cors::{Any, CorsLayer};

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
        .merge(momentum_api::routes::oauth::routes())
        .with_state(state.clone());

    // Build router - apply auth middleware only to routes that need it
    // P2.2 修复：通过 layer 注入 AuthConfig，避免 default() 密钥回退
    use momentum_api::middleware::auth::AuthConfig;
    let auth_config = AuthConfig::from_config(&core_config);

    let protected_routes = momentum_api::routes::create_router(state.clone())
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(state.db.clone()),
            momentum_api::middleware::auth::auth_middleware,
        ))
        .layer(axum::Extension(auth_config));

    let app = Router::new()
        // P3.1 修复：业务 API 挂载到 /v1 路径，支持版本演进
        .nest("/v1", protected_routes)
        // WebSocket 和 auth_routes 暂不版本化（保持向后兼容）
        // 未来可在 /v1/auth 中提供新版本认证
        .merge(auth_routes)
        .merge(websocket::create_websocket_routes().with_state(ws_state))
        .layer(cors)
        .layer(from_fn(request_tracking_middleware))
        .layer(from_fn(performance_monitoring_middleware))
        .layer(from_fn(momentum_api::middleware::logger::logger));

    // Start server
    let addr = config.server_address;
    tracing::info!("Server running at http://{}", addr);
    tracing::info!("WebSocket endpoint available at ws://{}/ws", addr);
    tracing::info!("Press Ctrl+C or send SIGTERM to shutdown gracefully");

    // P1.1 修复：使用 graceful shutdown
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shut down gracefully");
    Ok(())
}

/// 监听操作系统信号（Ctrl+C / SIGTERM）触发优雅关闭
///
/// 当接收到信号时：
/// 1. 停止接受新连接
/// 2. 等待正在进行的请求完成（默认超时 30s）
/// 3. 关闭所有 WebSocket 连接
/// 4. 归还数据库连接到池中
/// 5. 清理后台任务
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown...");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown...");
        }
    }
}