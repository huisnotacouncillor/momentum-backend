use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub message_type: MessageType,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text, Notification, SystemMessage, UserJoined, UserLeft,
    Ping, Pong, Error, Command, CommandResponse, InitialData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connected, Reconnecting, Disconnected, Suspended,
}

#[derive(Debug, Clone)]
pub struct ConnectedUser {
    pub user_id: Uuid,
    pub username: String,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub state: ConnectionState,
    pub message_queue: VecDeque<WebSocketMessage>,
    pub recovery_token: Option<String>,
    pub metadata: HashMap<String, String>,
    pub current_workspace_id: Option<Uuid>,
}

impl Default for ConnectedUser {
    fn default() -> Self {
        Self {
            user_id: Uuid::nil(),
            username: String::new(),
            connected_at: chrono::Utc::now(),
            last_ping: chrono::Utc::now(),
            state: ConnectionState::Disconnected,
            message_queue: VecDeque::new(),
            recovery_token: None,
            metadata: HashMap::new(),
            current_workspace_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecoveryInfo {
    pub user_id: Uuid,
    pub recovery_token: String,
    pub expires_at: DateTime<Utc>,
    pub pending_messages: VecDeque<WebSocketMessage>,
}
