use axum::extract::ws::{Message, WebSocket};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub id: String,
    pub message_type: MessageType,
    pub data: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub from_user_id: Option<Uuid>,
    pub to_user_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Text,
    Notification,
    SystemMessage,
    UserJoined,
    UserLeft,
    Ping,
    Pong,
    Error,
}

#[derive(Debug, Clone)]
pub struct ConnectedUser {
    pub user_id: Uuid,
    pub username: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_ping: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone)]
pub struct WebSocketManager {
    // 存储所有活跃连接
    connections: Arc<RwLock<HashMap<String, ConnectedUser>>>,
    // 广播通道
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
        }
    }

    // 添加新连接
    pub async fn add_connection(&self, connection_id: String, user: ConnectedUser) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), user.clone());

        info!(
            "User {} connected with connection_id: {}",
            user.username, connection_id
        );

        // 发送用户加入消息
        let join_message = WebSocketMessage {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::UserJoined,
            data: serde_json::json!({
                "user_id": user.user_id,
                "username": user.username,
                "message": format!("{} joined the chat", user.username)
            }),
            timestamp: chrono::Utc::now(),
            from_user_id: Some(user.user_id),
            to_user_id: None,
        };

        let _ = self.broadcast_tx.send(join_message);
    }

    // 移除连接
    pub async fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.remove(connection_id) {
            info!(
                "User {} disconnected with connection_id: {}",
                user.username, connection_id
            );

            // 发送用户离开消息
            let leave_message = WebSocketMessage {
                id: Uuid::new_v4().to_string(),
                message_type: MessageType::UserLeft,
                data: serde_json::json!({
                    "user_id": user.user_id,
                    "username": user.username,
                    "message": format!("{} left the chat", user.username)
                }),
                timestamp: chrono::Utc::now(),
                from_user_id: Some(user.user_id),
                to_user_id: None,
            };

            let _ = self.broadcast_tx.send(leave_message);
        }
    }

    // 获取连接用户信息
    pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectedUser> {
        let connections = self.connections.read().await;
        connections.get(connection_id).cloned()
    }

    // 更新连接的最后ping时间
    pub async fn update_ping(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.last_ping = chrono::Utc::now();
        }
    }

    // 获取所有在线用户
    pub async fn get_online_users(&self) -> Vec<ConnectedUser> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    // 获取在线用户数量
    pub async fn get_connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    // 广播消息给所有连接
    pub async fn broadcast_message(&self, message: WebSocketMessage) {
        if let Err(e) = self.broadcast_tx.send(message) {
            error!("Failed to broadcast message: {}", e);
        }
    }

    // 发送消息给特定用户
    pub async fn send_to_user(&self, user_id: Uuid, message: WebSocketMessage) {
        let connections = self.connections.read().await;
        let user_connections: Vec<_> = connections
            .iter()
            .filter(|(_, user)| user.user_id == user_id)
            .map(|(id, _)| id.clone())
            .collect();

        if !user_connections.is_empty() {
            if let Err(e) = self.broadcast_tx.send(message) {
                error!("Failed to send message to user {}: {}", user_id, e);
            }
        } else {
            warn!("User {} is not connected", user_id);
        }
    }

    // 获取广播接收器
    pub fn subscribe(&self) -> broadcast::Receiver<WebSocketMessage> {
        self.broadcast_tx.subscribe()
    }

    // 清理超时连接
    pub async fn cleanup_stale_connections(&self, timeout_minutes: i64) {
        let mut connections = self.connections.write().await;
        let cutoff_time = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes);

        let stale_connections: Vec<String> = connections
            .iter()
            .filter(|(_, user)| user.last_ping < cutoff_time)
            .map(|(id, _)| id.clone())
            .collect();

        for connection_id in stale_connections {
            if let Some(user) = connections.remove(&connection_id) {
                warn!("Removed stale connection for user: {}", user.username);
            }
        }
    }

    // 处理WebSocket连接
    pub async fn handle_socket(
        &self,
        mut socket: WebSocket,
        connection_id: String,
        user: ConnectedUser,
    ) {
        // 订阅广播消息
        let mut rx = self.subscribe();
        let user_id = user.user_id;

        // 添加连接
        self.add_connection(connection_id.clone(), user).await;

        // 发送连接成功消息
        let welcome_message = WebSocketMessage {
            id: Uuid::new_v4().to_string(),
            message_type: MessageType::SystemMessage,
            data: serde_json::json!({
                "message": "Connected successfully",
                "connection_id": connection_id,
                "online_users": self.get_online_users().await.len()
            }),
            timestamp: chrono::Utc::now(),
            from_user_id: None,
            to_user_id: Some(user_id),
        };

        if let Ok(msg_text) = serde_json::to_string(&welcome_message) {
            let _ = socket.send(Message::Text(msg_text)).await;
        }

        // 分离发送和接收
        let (mut sender, mut receiver) = socket.split();
        let manager = self.clone();
        let connection_id_clone = connection_id.clone();

        // 处理接收到的消息
        let recv_task = {
            let manager = manager.clone();
            let connection_id = connection_id.clone();
            tokio::spawn(async move {
                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text)
                            {
                                match ws_message.message_type {
                                    MessageType::Ping => {
                                        manager.update_ping(&connection_id).await;
                                        let pong = WebSocketMessage {
                                            id: Uuid::new_v4().to_string(),
                                            message_type: MessageType::Pong,
                                            data: serde_json::json!({"timestamp": chrono::Utc::now()}),
                                            timestamp: chrono::Utc::now(),
                                            from_user_id: None,
                                            to_user_id: Some(user_id),
                                        };
                                        manager.broadcast_message(pong).await;
                                    }
                                    MessageType::Text => {
                                        // 广播文本消息
                                        let mut broadcast_msg = ws_message;
                                        broadcast_msg.from_user_id = Some(user_id);
                                        broadcast_msg.timestamp = chrono::Utc::now();
                                        manager.broadcast_message(broadcast_msg).await;
                                    }
                                    _ => {
                                        // 处理其他类型的消息
                                        let mut broadcast_msg = ws_message;
                                        broadcast_msg.from_user_id = Some(user_id);
                                        broadcast_msg.timestamp = chrono::Utc::now();
                                        manager.broadcast_message(broadcast_msg).await;
                                    }
                                }
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!(
                                "WebSocket connection closed for connection_id: {}",
                                connection_id
                            );
                            break;
                        }
                        Err(e) => {
                            error!("WebSocket error for connection_id {}: {}", connection_id, e);
                            break;
                        }
                        _ => {}
                    }
                }
            })
        };

        // 处理广播消息
        let send_task = tokio::spawn(async move {
            while let Ok(message) = rx.recv().await {
                // 检查消息是否是发给这个用户的
                let should_send = match message.to_user_id {
                    Some(target_user_id) => target_user_id == user_id,
                    None => true, // 广播消息发给所有人
                };

                if should_send {
                    if let Ok(msg_text) = serde_json::to_string(&message) {
                        if sender.send(Message::Text(msg_text)).await.is_err() {
                            break;
                        }
                    }
                }
            }
        });

        // 等待任务完成
        tokio::select! {
            _ = recv_task => {},
            _ = send_task => {},
        }

        // 清理连接
        self.remove_connection(&connection_id_clone).await;
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}
