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
        self.subscription.subscribe(connection_id, topic).await
    }

    pub async fn unsubscribe(&self, connection_id: &str, topic: Topic) -> UnsubscribeResult {
        self.subscription.unsubscribe(connection_id, topic).await
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}
