// Legacy modules (kept for backward compatibility)
pub mod auth;
pub mod commands;
pub mod error_mapper;
pub mod handler;
pub mod manager;
pub mod monitoring;
pub mod rate_limiter;
pub mod retry_timeout;
pub mod security;
pub mod tests;

// New unified event system modules (temporarily commented out due to compilation issues)
// pub mod batch_processor;
// pub mod events;
// pub mod unified_manager;

// Re-export commonly used types for convenience
pub use auth::{AuthenticatedUser, WebSocketAuth, WebSocketAuthError, WebSocketAuthQuery};
pub use commands::{
    WebSocketCommand, WebSocketCommandError, WebSocketCommandHandler, WebSocketCommandResponse,
};
pub use error_mapper::{
    WebSocketError, WebSocketErrorCode, WebSocketErrorHandler, WebSocketErrorMapper,
};

// Legacy handler exports (backward compatibility)
pub use handler::{
    BroadcastMessageRequest, BroadcastMessageResponse, CleanupResponse, OnlineUserInfo,
    OnlineUsersResponse, SendMessageRequest, SendMessageResponse, WebSocketHandler, WebSocketState,
    WebSocketStats,
};
pub use manager::{ConnectedUser, MessageType, WebSocketManager, WebSocketMessage};
pub use monitoring::{
    ConnectionQuality, HealthCheck, HealthStatus, MonitoringConfig, MonitoringData,
    PerformanceMetrics, WebSocketMonitor,
};
pub use rate_limiter::{RateLimitConfig, RateLimitError, WebSocketRateLimiter};
pub use retry_timeout::{
    ConnectionHealthChecker, RetryConfig, RetryTimeoutError, RetryTimeoutManager, TimeoutConfig,
};
pub use security::{MessageSigner, SecureMessage, SecureMessageBuilder, SecurityError};

// New unified system exports (temporarily commented out due to compilation issues)
// pub use batch_processor::{
//     BatchConfig, BatchItem, BatchProcessor, BatchProcessorManager, BatchResult,
// };
// pub use events::{
//     Event, EventBuilder, EventMiddleware, GenericWebSocketEvent, WebSocketEvent,
//     business::{BusinessContext, BusinessEvent, BusinessEventHandler},
//     core::{EventContext, EventDispatcher, EventError, EventResult},
//     handlers::{ConnectionEventHandler, HandlerRegistry, MessageEventHandler, SystemEventHandler},
//     middleware::{MiddlewareChain, create_default_middleware_chain},
//     types::{EventConfig, EventMetrics, EventPriority, EventType},
// };
// pub use unified_manager::{
//     BroadcastMessage, BroadcastTarget, BroadcastType, ManagerConfig, UnifiedWebSocketManager,
// };

use crate::db::DbPool;
use std::sync::Arc;

// Temporarily commented out due to compilation issues
// /// Unified WebSocket state for the new system
// #[derive(Clone)]
// pub struct UnifiedWebSocketState {
//     pub manager: UnifiedWebSocketManager,
//     pub db: Arc<DbPool>,
// }

// impl UnifiedWebSocketState {
//     pub async fn new(db: Arc<DbPool>, config: ManagerConfig) -> Self {
//         let manager = UnifiedWebSocketManager::new(db.clone(), config).await;
//         Self { manager, db }
//     }

//     pub async fn get_stats(&self) -> serde_json::Value {
//         self.manager.get_health_status().await
//     }
// }

/// Legacy WebSocket state (backward compatibility)
pub fn create_websocket_state(db: Arc<DbPool>, config: &crate::config::Config) -> WebSocketState {
    let ws_manager = WebSocketManager::new();
    let message_signer = Arc::new(MessageSigner::new(config));
    let command_handler =
        WebSocketCommandHandler::new(db.clone()).with_message_signer(message_signer.clone());
    let rate_limiter = WebSocketRateLimiter::new(RateLimitConfig::default());
    let error_handler = WebSocketErrorHandler::new();
    let retry_timeout_manager =
        RetryTimeoutManager::new(RetryConfig::default(), TimeoutConfig::default());
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

// Temporarily commented out due to compilation issues
// /// Create unified WebSocket state with new event system
// pub async fn create_unified_websocket_state(
//     db: Arc<DbPool>,
//     config: ManagerConfig,
// ) -> UnifiedWebSocketState {
//     UnifiedWebSocketState::new(db, config).await
// }

/// Create WebSocket routes for legacy system
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

// Temporarily commented out due to compilation issues
// /// Create unified WebSocket routes with new event system
// pub fn create_unified_websocket_routes() -> axum::Router<UnifiedWebSocketState> {
//     // Implementation temporarily disabled
//     axum::Router::new()
// }

/// Background task to periodically clean up stale connections (legacy)
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
