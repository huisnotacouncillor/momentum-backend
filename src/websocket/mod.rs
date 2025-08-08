pub mod auth;
pub mod handler;
pub mod manager;

// Re-export commonly used types for convenience
pub use auth::{AuthenticatedUser, WebSocketAuth, WebSocketAuthError, WebSocketAuthQuery};
pub use handler::{
    BroadcastMessageRequest, BroadcastMessageResponse, CleanupResponse, OnlineUserInfo,
    OnlineUsersResponse, SendMessageRequest, SendMessageResponse, WebSocketHandler, WebSocketState,
    WebSocketStats,
};
pub use manager::{ConnectedUser, MessageType, WebSocketManager, WebSocketMessage};

use crate::db::DbPool;
use std::sync::Arc;

/// Initialize WebSocket services
pub fn create_websocket_state(db: Arc<DbPool>) -> WebSocketState {
    let ws_manager = WebSocketManager::new();

    WebSocketState { db, ws_manager }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all re-exports are accessible
        let _: Option<WebSocketManager> = None;
        let _: Option<WebSocketHandler> = None;
        let _: Option<WebSocketAuth> = None;
    }
}
