pub mod auth;
pub mod commands;
pub mod error_mapper;
pub mod handler;
pub mod manager;
pub mod rate_limiter;
pub mod retry_timeout;
pub mod monitoring;
pub mod security;
pub mod tests;

// Re-export commonly used types for convenience
pub use auth::{AuthenticatedUser, WebSocketAuth, WebSocketAuthError, WebSocketAuthQuery};
pub use commands::{WebSocketCommand, WebSocketCommandHandler, WebSocketCommandResponse, WebSocketCommandError};
pub use error_mapper::{WebSocketErrorMapper, WebSocketErrorHandler, WebSocketError, WebSocketErrorCode};
pub use handler::{
    BroadcastMessageRequest, BroadcastMessageResponse, CleanupResponse, OnlineUserInfo,
    OnlineUsersResponse, SendMessageRequest, SendMessageResponse, WebSocketHandler, WebSocketState,
    WebSocketStats,
};
pub use manager::{ConnectedUser, MessageType, WebSocketManager, WebSocketMessage};
pub use rate_limiter::{WebSocketRateLimiter, RateLimitConfig, RateLimitError};
pub use retry_timeout::{RetryTimeoutManager, RetryConfig, TimeoutConfig, RetryTimeoutError, ConnectionHealthChecker};
pub use monitoring::{WebSocketMonitor, MonitoringConfig, PerformanceMetrics, ConnectionQuality, HealthStatus, HealthCheck, MonitoringData};
pub use security::{SecureMessage, MessageSigner, SecurityError, SecureMessageBuilder};

use crate::db::DbPool;
use std::sync::Arc;

/// Initialize WebSocket services
pub fn create_websocket_state(db: Arc<DbPool>, config: &crate::config::Config) -> WebSocketState {
    let ws_manager = WebSocketManager::new();
    let message_signer = Arc::new(MessageSigner::new(config));
    let command_handler = WebSocketCommandHandler::new(db.clone())
        .with_message_signer(message_signer.clone());
    let rate_limiter = WebSocketRateLimiter::new(RateLimitConfig::default());
    let error_handler = WebSocketErrorHandler::new();
    let retry_timeout_manager = RetryTimeoutManager::new(RetryConfig::default(), TimeoutConfig::default());
    let monitor = WebSocketMonitor::new(MonitoringConfig::default());

    // 启动清理任务
    tokio::spawn({
        let signer = message_signer.clone();
        async move {
            signer.start_cleanup_task().await;
        }
    });

    WebSocketState {
        db,
        ws_manager,
        command_handler,
        rate_limiter,
        error_handler,
        retry_timeout_manager,
        monitor,
        message_signer: (*message_signer).clone(),
    }
}

/// Create WebSocket routes
pub fn create_websocket_routes() -> axum::Router<WebSocketState> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/ws", get(WebSocketHandler::websocket_handler))
        .route("/ws/online", get(WebSocketHandler::get_online_users))
        .route("/ws/stats", get(WebSocketHandler::get_websocket_stats))
        .route("/ws/send", post(WebSocketHandler::send_message_to_user))
        .route("/ws/broadcast", post(WebSocketHandler::broadcast_message))
        .route("/ws/cleanup", post(WebSocketHandler::cleanup_connections))
}

/// Background task to periodically clean up stale connections
pub async fn start_connection_cleanup_task(ws_manager: WebSocketManager) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300)); // 5 minutes

    loop {
        interval.tick().await;
        tracing::debug!("Running WebSocket connection cleanup");

        // Clean up connections that haven't pinged in the last 10 minutes
        ws_manager.cleanup_stale_connections(10).await;
    }
}

// tests are in separate module `tests.rs`
