pub mod broadcast;
pub mod connection;
pub mod offline_queue;
pub mod recovery;
pub mod subscription;
pub mod state;

pub use broadcast::BroadcastManager;
pub use connection::ConnectionManager;
pub use offline_queue::OfflineQueueManager;
pub use recovery::RecoveryManager;
pub use state::{ConnectedUser, ConnectionRecoveryInfo, ConnectionState, MessageType, WebSocketMessage};
pub use subscription::{SubscriptionManager, SubscribeResult, Topic, UnsubscribeResult};

use std::sync::Arc;
use uuid::Uuid;
use std::collections::VecDeque;

// ============ Facade ============

#[derive(Clone)]
pub struct WebSocketManager {
    pub(crate) conn: Arc<ConnectionManager>,
    pub(crate) broadcast: Arc<BroadcastManager>,
    pub(crate) recovery: Arc<RecoveryManager>,
    pub(crate) offline: Arc<OfflineQueueManager>,
    pub(crate) subscription: Arc<SubscriptionManager>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let conn = Arc::new(ConnectionManager::new());
        let broadcast = Arc::new(BroadcastManager::new());
        let recovery = Arc::new(RecoveryManager::new());
        let offline = Arc::new(OfflineQueueManager::new(conn.clone()));
        let subscription = Arc::new(SubscriptionManager::new());

        Self { conn, broadcast, recovery, offline, subscription }
    }

    // === Connection (委托) ===
    pub async fn add_connection(
        &self,
        connection_id: String,
        user: ConnectedUser,
        db: Option<&Arc<momentum_core::db::DbPool>>,
        asset_helper: Option<&Arc<momentum_core::utils::AssetUrlHelper>>,
    ) {
        self.broadcast.register_sender(connection_id.clone()).await;
        self.conn.add_connection(connection_id, user, db, asset_helper).await;
    }

    pub async fn remove_connection(&self, connection_id: &str) {
        self.broadcast.unregister_sender(connection_id).await;
        self.conn.remove_connection(connection_id).await;
    }

    pub async fn suspend_connection(&self, connection_id: &str) {
        self.conn.suspend_connection(connection_id).await;
    }

    pub async fn resume_connection(&self, connection_id: &str) {
        self.conn.resume_connection(connection_id).await;
    }

    pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectedUser> {
        self.conn.get_connection(connection_id).await
    }

    pub async fn update_ping(&self, connection_id: &str) {
        self.conn.update_ping(connection_id).await;
    }

    pub async fn get_online_users(&self) -> Vec<ConnectedUser> {
        self.conn.get_online_users().await
    }

    pub async fn get_connection_count(&self) -> usize {
        self.conn.get_connection_count().await
    }

    pub async fn cleanup_stale_connections(&self, timeout_minutes: i64) -> usize {
        self.conn.cleanup_stale_connections(timeout_minutes).await
    }

    // === Broadcast (委托) ===
    pub async fn broadcast_message(&self, message: WebSocketMessage) {
        self.broadcast.broadcast_message(message).await;
    }

    pub async fn broadcast_to_workspace(&self, workspace_id: Uuid, message: WebSocketMessage) {
        let workspace_users = self.conn.get_connections_in_workspace(workspace_id).await;
        if !workspace_users.is_empty() {
            self.broadcast.broadcast_to_workspace(workspace_id, message).await;
        }
    }

    pub async fn direct_send(&self, connection_id: &str, message: WebSocketMessage) -> Result<(), String> {
        self.broadcast.direct_send(connection_id, message).await
    }

    pub fn get_broadcast_receiver(&self) -> tokio::sync::broadcast::Receiver<WebSocketMessage> {
        self.broadcast.get_broadcast_receiver()
    }

    pub async fn broadcast_issue_event(&self, event: super::issue_events::IssueEvent) {
        self.broadcast.broadcast_issue_event(event).await;
    }

    pub async fn broadcast_issue_event_to_workspace(&self, workspace_id: Uuid, event: super::issue_events::IssueEvent) {
        let workspace_users = self.conn.get_connections_in_workspace(workspace_id).await;
        if !workspace_users.is_empty() {
            self.broadcast.broadcast_issue_event_to_workspace(workspace_id, event).await;
        }
    }

    // === Recovery (委托) ===
    pub async fn recover_connection(&self, user_id: Uuid, recovery_token: &str) -> Option<ConnectedUser> {
        self.recovery.recover_connection(user_id, recovery_token).await
    }

    // === Offline Queue (委托) ===
    pub async fn add_offline_message(&self, user_id: Uuid, message: WebSocketMessage) {
        self.offline.add_offline_message(user_id, message).await;
    }

    pub async fn get_offline_messages(&self, user_id: Uuid) -> VecDeque<WebSocketMessage> {
        self.offline.get_offline_messages(user_id).await
    }

    // === Subscription (委托) ===
    pub async fn subscribe(&self, connection_id: &str, topic: Topic) -> SubscribeResult {
        self.subscription.subscribe(connection_id, &[topic]).await
    }

    pub async fn unsubscribe(&self, connection_id: &str, topic: Topic) -> UnsubscribeResult {
        self.subscription.unsubscribe(connection_id, &[topic]).await
    }

    // === User messaging ===
    pub async fn send_to_user(&self, user_id: Uuid, message: WebSocketMessage) {
        // Find connection by user_id and send message
        let connections = self.conn.get_connections_by_user(user_id).await;
        for conn_id in connections {
            let _ = self.direct_send(&conn_id, message.clone()).await;
        }
    }

    /// Handle a WebSocket connection (delegated to connection manager)
    pub async fn handle_socket(
        &self,
        mut socket: axum::extract::ws::WebSocket,
        connection_id: String,
        user: ConnectedUser,
        command_handler: Option<crate::websocket::WebSocketCommandHandler>,
        monitor: Option<crate::websocket::WebSocketMonitor>,
        db: Option<Arc<momentum_core::db::DbPool>>,
        asset_helper: Option<Arc<momentum_core::utils::AssetUrlHelper>>,
    ) {
        use axum::extract::ws::Message;
        use futures_util::{SinkExt, StreamExt};

        // Add connection
        self.add_connection(
            connection_id.clone(),
            user.clone(),
            db.as_ref(),
            asset_helper.as_ref(),
        )
        .await;

        // Record connection in monitor
        if let Some(ref monitor) = monitor {
            monitor.record_connection(user.user_id, connection_id.clone()).await;
        }

        // Subscribe to broadcast messages
        let mut rx = self.get_broadcast_receiver();

        // Send welcome message
        let welcome = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: MessageType::SystemMessage,
            data: serde_json::json!({
                "message": "Connected successfully",
                "connection_id": connection_id
            }),
            timestamp: Some(chrono::Utc::now()),
        };
        if let Ok(text) = serde_json::to_string(&welcome) {
            let _ = socket.send(Message::Text(text)).await;
        }

        // Split socket
        let (mut sender, mut receiver) = socket.split();
        let manager = self.clone();
        let connection_id_clone = connection_id.clone();

        // Spawn receive task
        tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                if let Ok(Message::Text(text)) = msg {
                    if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text) {
                        match ws_message.message_type {
                            MessageType::Ping => {
                                let pong = WebSocketMessage {
                                    id: Some(Uuid::new_v4().to_string()),
                                    message_type: MessageType::Pong,
                                    data: serde_json::json!({"timestamp": chrono::Utc::now()}),
                                    timestamp: Some(chrono::Utc::now()),
                                };
                                manager.broadcast_message(pong).await;
                            }
                            MessageType::Command => {
                                if let Some(ref handler) = command_handler {
                                    if let Ok(command) = serde_json::from_value(ws_message.data.clone()) {
                                        let auth_user = crate::websocket::auth::AuthenticatedUser {
                                            user_id: user.user_id,
                                            username: user.username.clone(),
                                            email: String::new(),
                                            name: user.username.clone(),
                                            avatar_url: None,
                                            current_workspace_id: user.current_workspace_id,
                                        };
                                        let response = handler.handle_command(
                                            command,
                                            &auth_user,
                                            &connection_id_clone,
                                        ).await;
                                        let resp_msg = WebSocketMessage {
                                            id: Some(Uuid::new_v4().to_string()),
                                            message_type: MessageType::CommandResponse,
                                            data: serde_json::to_value(&response).unwrap(),
                                            timestamp: Some(chrono::Utc::now()),
                                        };
                                        manager.broadcast_message(resp_msg).await;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // Cleanup on disconnect
            manager.remove_connection(&connection_id_clone).await;
        });

        // Forward broadcast messages to socket
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                if let Ok(text) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(text)).await.is_err() {
                        break;
                    }
                }
            }
        });
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}
