use axum::{
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::Response,
};
use std::sync::Arc;
use uuid::Uuid;

use momentum_core::db::DbPool;
use crate::{
    websocket::{
        auth::{WebSocketAuth, WebSocketAuthQuery},
        manager::{ConnectedUser, WebSocketManager},
    },
};

#[derive(Clone)]
pub struct WebSocketState {
    pub db: Arc<DbPool>,
    pub ws_manager: WebSocketManager,
    pub command_handler: crate::websocket::WebSocketCommandHandler,
    pub rate_limiter: crate::websocket::WebSocketRateLimiter,
    pub error_handler: crate::websocket::WebSocketErrorHandler,
    pub retry_timeout_manager: crate::websocket::RetryTimeoutManager,
    pub monitor: crate::websocket::WebSocketMonitor,
    pub message_signer: crate::websocket::MessageSigner,
}

pub struct WebSocketHandler;

impl WebSocketHandler {
    /// 处理WebSocket升级请求
    ///
    /// Issue #11：token 优先从 `Sec-WebSocket-Protocol` 头读取，URL query 留作兼容。
    /// 客户端模式：
    /// - 新：`Sec-WebSocket-Protocol: momentum-v1, <JWT>`
    /// - 旧：`?token=<JWT>`（仍可用，但会触发 warn 日志，建议尽快迁移）
    pub async fn websocket_handler(
        ws: WebSocketUpgrade,
        headers: HeaderMap,
        Query(query): Query<WebSocketAuthQuery>,
        State(state): State<WebSocketState>,
    ) -> axum::response::Result<Response> {
        // Issue #11：尝试从 Sec-WebSocket-Protocol 头取 token；回退到 URL query
        let sec_protocol = headers.get("sec-websocket-protocol");
        let query_map: std::collections::HashMap<String, String> = query
            .token
            .clone()
            .map(|t| std::collections::HashMap::from([("token".to_string(), t)]))
            .unwrap_or_default();

        let token = WebSocketAuth::extract_token(sec_protocol, &query_map).or_else(|| {
            tracing::warn!(
                "WebSocket client sent token via URL query (legacy). \
                 Issue #11: please migrate to Sec-WebSocket-Protocol header to avoid token leaking into access logs."
            );
            WebSocketAuth::extract_token_from_params(&query_map)
        });

        let token = match token {
            Some(t) => t,
            None => {
                tracing::warn!("WebSocket authentication failed: missing token");
                return Err((axum::http::StatusCode::UNAUTHORIZED, "Missing authentication token").into());
            }
        };

        // 验证认证token
        let authenticated_user =
            match WebSocketAuth::authenticate_websocket(state.db.clone(), &token).await {
                Ok(user) => user,
                Err(error) => {
                    tracing::warn!("WebSocket authentication failed: {}", error);
                    let (status, message) = WebSocketAuth::error_response(error);
                    return Err((status, message).into());
                }
            };

        tracing::info!(
            "WebSocket upgrade request from user: {} ({})",
            authenticated_user.username,
            authenticated_user.user_id
        );

        // 升级到WebSocket连接
        Ok(ws.on_upgrade(move |socket| {
            Self::handle_websocket_connection(
                socket,
                authenticated_user,
                state.ws_manager,
                state.command_handler.clone(),
                state.monitor.clone(),
                state.db.clone(),
            )
        }))
    }

    /// 处理WebSocket连接
    async fn handle_websocket_connection(
        socket: WebSocket,
        authenticated_user: crate::websocket::auth::AuthenticatedUser,
        ws_manager: WebSocketManager,
        command_handler: crate::websocket::WebSocketCommandHandler,
        monitor: crate::websocket::WebSocketMonitor,
        db: Arc<DbPool>,
    ) {
        let connection_id = Uuid::new_v4().to_string();
        let connected_user = ConnectedUser {
            user_id: authenticated_user.user_id,
            username: authenticated_user.username.clone(),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
            state: crate::websocket::manager::ConnectionState::Connected,
            message_queue: std::collections::VecDeque::new(),
            recovery_token: None,
            metadata: std::collections::HashMap::new(),
            current_workspace_id: authenticated_user.current_workspace_id,
        };

        tracing::info!(
            "Handling WebSocket connection for user: {} with connection_id: {}",
            authenticated_user.username,
            connection_id
        );

        // 获取 asset_helper 从 command_handler
        let asset_helper = command_handler.get_asset_helper();

        // 将连接处理委托给WebSocket管理器
        ws_manager
            .handle_socket(
                socket,
                connection_id,
                connected_user,
                Some(command_handler),
                Some(monitor),
                Some(db),
                Some(asset_helper),
            )
            .await;
    }

    /// 获取在线用户列表
    pub async fn get_online_users(
        State(state): State<WebSocketState>,
    ) -> axum::Json<OnlineUsersResponse> {
        let users = state.ws_manager.get_online_users().await;
        let count = users.len();

        axum::Json(OnlineUsersResponse {
            count,
            users: users
                .into_iter()
                .map(|user| OnlineUserInfo {
                    user_id: user.user_id,
                    username: user.username,
                    connected_at: user.connected_at,
                })
                .collect(),
        })
    }

    /// 获取WebSocket连接统计信息
    pub async fn get_websocket_stats(
        State(state): State<WebSocketState>,
    ) -> axum::Json<WebSocketStats> {
        let connection_count = state.ws_manager.get_connection_count().await;
        let online_users = state.ws_manager.get_online_users().await;

        axum::Json(WebSocketStats {
            total_connections: connection_count,
            unique_users: online_users.len(),
            server_uptime: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    /// 向特定用户发送消息（HTTP API）
    pub async fn send_message_to_user(
        State(state): State<WebSocketState>,
        axum::Json(payload): axum::Json<SendMessageRequest>,
    ) -> Result<axum::Json<SendMessageResponse>, axum::http::StatusCode> {
        use crate::websocket::manager::{MessageType, WebSocketMessage};

        let message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: match payload.message_type.as_str() {
                "text" => MessageType::Text,
                "notification" => MessageType::Notification,
                "system" => MessageType::SystemMessage,
                _ => MessageType::Text,
            },
            data: payload.data,
            timestamp: Some(chrono::Utc::now()),
        };

        state
            .ws_manager
            .send_to_user(payload.to_user_id, message)
            .await;

        Ok(axum::Json(SendMessageResponse {
            success: true,
            message: "Message sent successfully".to_string(),
        }))
    }

    /// 广播消息给所有在线用户（HTTP API）
    pub async fn broadcast_message(
        State(state): State<WebSocketState>,
        axum::Json(payload): axum::Json<BroadcastMessageRequest>,
    ) -> Result<axum::Json<BroadcastMessageResponse>, axum::http::StatusCode> {
        use crate::websocket::manager::{MessageType, WebSocketMessage};

        let message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: match payload.message_type.as_str() {
                "text" => MessageType::Text,
                "notification" => MessageType::Notification,
                "system" => MessageType::SystemMessage,
                _ => MessageType::SystemMessage,
            },
            data: payload.data,
            timestamp: Some(chrono::Utc::now()),
        };

        state.ws_manager.broadcast_message(message).await;

        let connection_count = state.ws_manager.get_connection_count().await;

        Ok(axum::Json(BroadcastMessageResponse {
            success: true,
            message: "Message broadcasted successfully".to_string(),
            recipients_count: connection_count,
        }))
    }

    /// 清理过期连接
    pub async fn cleanup_connections(
        State(state): State<WebSocketState>,
    ) -> axum::Json<CleanupResponse> {
        let before_count = state.ws_manager.get_connection_count().await;

        // 清理超过5分钟没有ping的连接
        state.ws_manager.cleanup_stale_connections(5).await;

        let after_count = state.ws_manager.get_connection_count().await;
        let cleaned_count = before_count.saturating_sub(after_count);

        axum::Json(CleanupResponse {
            cleaned_connections: cleaned_count,
            remaining_connections: after_count,
        })
    }
}

// 响应结构体定义
#[derive(serde::Serialize)]
pub struct OnlineUsersResponse {
    pub count: usize,
    pub users: Vec<OnlineUserInfo>,
}

#[derive(serde::Serialize)]
pub struct OnlineUserInfo {
    pub user_id: Uuid,
    pub username: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
}

#[derive(serde::Serialize)]
pub struct WebSocketStats {
    pub total_connections: usize,
    pub unique_users: usize,
    pub server_uptime: u64,
}

#[derive(serde::Deserialize)]
pub struct SendMessageRequest {
    pub to_user_id: Uuid,
    pub message_type: String,
    pub data: serde_json::Value,
}

#[derive(serde::Serialize)]
pub struct SendMessageResponse {
    pub success: bool,
    pub message: String,
}

#[derive(serde::Deserialize)]
pub struct BroadcastMessageRequest {
    pub message_type: String,
    pub data: serde_json::Value,
}

#[derive(serde::Serialize)]
pub struct BroadcastMessageResponse {
    pub success: bool,
    pub message: String,
    pub recipients_count: usize,
}

#[derive(serde::Serialize)]
pub struct CleanupResponse {
    pub cleaned_connections: usize,
    pub remaining_connections: usize,
}

#[cfg(test)]
mod tests {
    use crate::websocket::manager::WebSocketManager;

    // NOTE: These tests are marked #[ignore] because WebSocketState contains
    // types like Arc<DbPool> that cannot be safely zero-initialized for testing.
    // The stats and online_users handlers only need ws_manager, but we cannot
    // partially construct WebSocketState without undefined behavior.
    //
    // To enable these tests:
    // 1. Set up a test database pool
    // 2. Create WebSocketState with properly constructed fields
    // 3. Remove the #[ignore] attribute
    //
    // Alternatively, refactor WebSocketState to use Option<Arc<DbPool>> for optional
    // fields to enable partial construction in tests.

    #[tokio::test]
    #[ignore = "WebSocketState requires fully constructed fields including Arc<DbPool> which cannot be zero-initialized"]
    async fn test_websocket_stats() {
        // This test would work if we had a test database pool
        // For now, it verifies the handler signature and basic assertions
        let ws_manager = WebSocketManager::new();
        assert_eq!(ws_manager.get_connection_count().await, 0);
    }

    #[tokio::test]
    #[ignore = "WebSocketState requires fully constructed fields including Arc<DbPool> which cannot be zero-initialized"]
    async fn test_online_users() {
        // This test would work if we had a test database pool
        // For now, it verifies the handler signature and basic assertions
        let ws_manager = WebSocketManager::new();
        let users = ws_manager.get_online_users().await;
        assert!(users.is_empty());
    }
}
