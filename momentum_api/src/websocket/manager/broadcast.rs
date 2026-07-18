use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, warn};
use uuid::Uuid;

use super::state::{MessageType, WebSocketMessage};
use crate::websocket::issue_events::IssueEvent;

pub struct BroadcastManager {
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
    direct_senders: Arc<RwLock<HashMap<String, broadcast::Sender<WebSocketMessage>>>>,
}

impl BroadcastManager {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            broadcast_tx,
            direct_senders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_sender(&self, connection_id: String) {
        let (tx, _) = broadcast::channel(100);
        let mut senders = self.direct_senders.write().await;
        senders.insert(connection_id, tx);
    }

    pub async fn unregister_sender(&self, connection_id: &str) {
        let mut senders = self.direct_senders.write().await;
        senders.remove(connection_id);
    }

    pub async fn broadcast_message(&self, message: WebSocketMessage) {
        if let Err(e) = self.broadcast_tx.send(message) {
            error!("📢 WebSocket Failed to broadcast message: {}", e);
        }
    }

    pub async fn broadcast_to_workspace(&self, workspace_id: Uuid, message: WebSocketMessage) {
        if let Err(e) = self.broadcast_tx.send(message) {
            error!(
                "📢 WebSocket Failed to broadcast to workspace {}: {}",
                workspace_id, e
            );
        }
    }

    pub async fn direct_send(
        &self,
        connection_id: &str,
        message: WebSocketMessage,
    ) -> Result<(), String> {
        let senders = self.direct_senders.read().await;
        if let Some(tx) = senders.get(connection_id) {
            tx.send(message)
                .map(|_| ())
                .map_err(|e| format!("direct_send failed: {}", e))
        } else {
            Err(format!("connection {} not found", connection_id))
        }
    }

    pub fn get_broadcast_receiver(&self) -> broadcast::Receiver<WebSocketMessage> {
        self.broadcast_tx.subscribe()
    }

    pub async fn broadcast_issue_event(&self, event: IssueEvent) {
        let message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: MessageType::Notification,
            data: serde_json::json!({
                "event": event.event_name(),
                "payload": event,
            }),
            timestamp: Some(chrono::Utc::now()),
        };
        self.broadcast_message(message).await;
    }

    pub async fn broadcast_issue_event_to_workspace(&self, workspace_id: Uuid, event: IssueEvent) {
        let message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: MessageType::Notification,
            data: serde_json::json!({
                "event": event.event_name(),
                "payload": event,
                "workspace_id": workspace_id,
            }),
            timestamp: Some(chrono::Utc::now()),
        };
        self.broadcast_to_workspace(workspace_id, message).await;
    }
}

impl Default for BroadcastManager {
    fn default() -> Self {
        Self::new()
    }
}
