use axum::extract::ws::{Message, WebSocket};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub message_type: MessageType,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    Command,         // 新增命令类型
    CommandResponse, // 新增命令响应类型
}

/// 连接状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Connected,    // 正常连接
    Reconnecting, // 重连中
    Disconnected, // 已断开
    Suspended,    // 暂停（临时断开）
}

/// 连接信息
#[derive(Debug, Clone)]
pub struct ConnectedUser {
    pub user_id: Uuid,
    pub username: String,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_ping: chrono::DateTime<chrono::Utc>,
    pub state: ConnectionState,
    pub subscriptions: HashSet<String>,            // 订阅的主题
    pub message_queue: VecDeque<WebSocketMessage>, // 离线消息队列
    pub recovery_token: Option<String>,            // 恢复令牌
    pub metadata: HashMap<String, String>,         // 连接元数据
    pub current_workspace_id: Option<Uuid>,
}

/// 连接恢复信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecoveryInfo {
    pub user_id: Uuid,
    pub recovery_token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub subscriptions: HashSet<String>,
    pub pending_messages: VecDeque<WebSocketMessage>,
}

#[derive(Clone)]
pub struct WebSocketManager {
    // 存储所有活跃连接
    connections: Arc<RwLock<HashMap<String, ConnectedUser>>>,
    // 广播通道
    broadcast_tx: broadcast::Sender<WebSocketMessage>,
    // 连接恢复信息
    recovery_info: Arc<RwLock<HashMap<Uuid, ConnectionRecoveryInfo>>>,
    // 订阅管理
    subscriptions: Arc<RwLock<HashMap<String, HashSet<Uuid>>>>, // topic -> user_ids
    // 配置
    max_queue_size: usize,
    recovery_token_ttl: Duration,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (broadcast_tx, _) = broadcast::channel(1000);
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            broadcast_tx,
            recovery_info: Arc::new(RwLock::new(HashMap::new())),
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            max_queue_size: 100,
            recovery_token_ttl: Duration::from_secs(300), // 5分钟
        }
    }

    /// 完善消息 - 自动生成ID和timestamp
    fn complete_message(&self, mut message: WebSocketMessage) -> WebSocketMessage {
        if message.id.is_none() {
            message.id = Some(Uuid::new_v4().to_string());
        }
        if message.timestamp.is_none() {
            message.timestamp = Some(chrono::Utc::now());
        }
        message
    }

    // 添加新连接
    pub async fn add_connection(&self, connection_id: String, user: ConnectedUser) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), user.clone());

        // 更新订阅信息
        let mut subscriptions = self.subscriptions.write().await;
        for topic in &user.subscriptions {
            subscriptions
                .entry(topic.clone())
                .or_insert_with(HashSet::new)
                .insert(user.user_id);
        }

        info!(
            "🔌 WebSocket User {} connected with connection ID {}",
            user.username, connection_id
        );

        // 发送用户加入消息
        let join_message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: MessageType::UserJoined,
            data: serde_json::json!({
                "user_id": user.user_id,
                "username": user.username,
                "connected_at": user.connected_at
            }),
            timestamp: Some(chrono::Utc::now()),
        };

        let _ = self.broadcast_tx.send(join_message);
    }

    // 移除连接
    pub async fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.remove(connection_id) {
            info!(
                "🔌 WebSocket User {} disconnected with connection_id: {}",
                user.username, connection_id
            );

            // 发送用户离开消息
            let leave_message = WebSocketMessage {
                id: Some(Uuid::new_v4().to_string()),
                message_type: MessageType::UserLeft,
                data: serde_json::json!({
                    "user_id": user.user_id,
                    "username": user.username,
                    "message": format!("{} left the chat", user.username)
                }),
                timestamp: Some(chrono::Utc::now()),
            };

            let _ = self.broadcast_tx.send(leave_message);

            // 创建恢复信息
            self.create_recovery_info(&user).await;
        }
    }

    /// 创建连接恢复信息
    async fn create_recovery_info(&self, user: &ConnectedUser) {
        let recovery_token = Uuid::new_v4().to_string();
        let expires_at =
            chrono::Utc::now() + chrono::Duration::from_std(self.recovery_token_ttl).unwrap();

        let recovery_info = ConnectionRecoveryInfo {
            user_id: user.user_id,
            recovery_token: recovery_token.clone(),
            expires_at,
            subscriptions: user.subscriptions.clone(),
            pending_messages: user.message_queue.clone(),
        };

        let mut recovery_map = self.recovery_info.write().await;
        recovery_map.insert(user.user_id, recovery_info);

        info!(
            "🔄 WebSocket Created recovery info for user {} with token {}",
            user.username, recovery_token
        );
    }

    /// 恢复连接
    pub async fn recover_connection(
        &self,
        user_id: Uuid,
        recovery_token: &str,
    ) -> Option<ConnectedUser> {
        let recovery_map = self.recovery_info.write().await;

        if let Some(recovery_info) = recovery_map.get(&user_id) {
            if recovery_info.recovery_token == recovery_token
                && recovery_info.expires_at > chrono::Utc::now()
            {
                // 恢复连接信息
                let mut connections = self.connections.write().await;
                if let Some(user) = connections.get_mut(&user_id.to_string()) {
                    user.state = ConnectionState::Connected;
                    user.subscriptions = recovery_info.subscriptions.clone();
                    user.message_queue = recovery_info.pending_messages.clone();
                    user.recovery_token = None;

                    // 更新订阅信息
                    let mut subscriptions = self.subscriptions.write().await;
                    for topic in &user.subscriptions {
                        subscriptions
                            .entry(topic.clone())
                            .or_insert_with(HashSet::new)
                            .insert(user_id);
                    }

                    info!(
                        "🔄 WebSocket Recovered connection for user {}",
                        user.username
                    );
                    return Some(user.clone());
                }
            }
        }

        None
    }

    /// 暂停连接（临时断开）
    pub async fn suspend_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.state = ConnectionState::Suspended;
            info!(
                "⏸️ WebSocket Suspended connection for user {}",
                user.username
            );
        }
    }

    /// 恢复暂停的连接
    pub async fn resume_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.state = ConnectionState::Connected;
            info!("▶️ WebSocket Resumed connection for user {}", user.username);
        }
    }

    /// 添加离线消息
    pub async fn add_offline_message(&self, user_id: Uuid, message: WebSocketMessage) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(&user_id.to_string()) {
            if user.state == ConnectionState::Connected {
                // 用户在线，直接发送
                return;
            }

            // 添加到离线消息队列
            user.message_queue.push_back(message);
            if user.message_queue.len() > self.max_queue_size {
                user.message_queue.pop_front();
            }

            info!(
                "📨 WebSocket Added offline message for user {}",
                user.username
            );
        }
    }

    /// 获取离线消息
    pub async fn get_offline_messages(&self, user_id: Uuid) -> VecDeque<WebSocketMessage> {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(&user_id.to_string()) {
            let messages = user.message_queue.clone();
            user.message_queue.clear();
            messages
        } else {
            VecDeque::new()
        }
    }

    /// 订阅主题
    pub async fn subscribe(&self, user_id: Uuid, topic: String) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(&user_id.to_string()) {
            user.subscriptions.insert(topic.clone());
        }

        let mut subscriptions = self.subscriptions.write().await;
        subscriptions
            .entry(topic)
            .or_insert_with(HashSet::new)
            .insert(user_id);
    }

    /// 取消订阅主题
    pub async fn unsubscribe(&self, user_id: Uuid, topic: String) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(&user_id.to_string()) {
            user.subscriptions.remove(&topic);
        }

        let mut subscriptions = self.subscriptions.write().await;
        if let Some(subscribers) = subscriptions.get_mut(&topic) {
            subscribers.remove(&user_id);
            if subscribers.is_empty() {
                subscriptions.remove(&topic);
            }
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
            error!("📢 WebSocket Failed to broadcast message: {}", e);
        }
    }

    // 基于workspace广播消息
    pub async fn broadcast_to_workspace(&self, workspace_id: Uuid, message: WebSocketMessage) {
        let connections = self.connections.read().await;
        let workspace_users: Vec<_> = connections
            .iter()
            .filter(|(_, user)| user.current_workspace_id == Some(workspace_id))
            .map(|(id, _)| id.clone())
            .collect();

        if !workspace_users.is_empty() {
            info!(
                "📢 WebSocket Broadcasting to workspace {} ({} users)",
                workspace_id,
                workspace_users.len()
            );
            if let Err(e) = self.broadcast_tx.send(message) {
                error!(
                    "📢 WebSocket Failed to broadcast to workspace {}: {}",
                    workspace_id, e
                );
            }
        } else {
            warn!("⚠️ WebSocket No users found in workspace {}", workspace_id);
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
                error!(
                    "📤 WebSocket Failed to send message to user {}: {}",
                    user_id, e
                );
            }
        } else {
            warn!("⚠️ WebSocket User {} is not connected", user_id);
        }
    }

    // 获取广播接收器
    pub fn get_broadcast_receiver(&self) -> broadcast::Receiver<WebSocketMessage> {
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
                warn!(
                    "🧹 WebSocket Removed stale connection for user: {}",
                    user.username
                );
            }
        }
    }

    // 处理WebSocket连接
    pub async fn handle_socket(
        &self,
        mut socket: WebSocket,
        connection_id: String,
        user: ConnectedUser,
        command_handler: Option<crate::websocket::WebSocketCommandHandler>,
        monitor: Option<crate::websocket::WebSocketMonitor>,
    ) {
        // 订阅广播消息
        let mut rx = self.get_broadcast_receiver();
        let user_id = user.user_id;
        let username = user.username.clone();

        // 添加连接
        self.add_connection(connection_id.clone(), user.clone())
            .await;

        // 记录连接监控
        if let Some(ref monitor) = monitor {
            monitor
                .record_connection(user_id, connection_id.clone())
                .await;
        }

        // 发送连接成功消息
        let welcome_message = WebSocketMessage {
            id: Some(Uuid::new_v4().to_string()),
            message_type: MessageType::SystemMessage,
            data: serde_json::json!({
                "message": "Connected successfully",
                "connection_id": connection_id,
                "online_users": self.get_online_users().await.len()
            }),
            timestamp: Some(chrono::Utc::now()),
        };

        if let Ok(msg_text) = serde_json::to_string(&welcome_message) {
            let _ = socket.send(Message::Text(msg_text)).await;
        }

        // 分离发送和接收
        let (mut sender, mut receiver) = socket.split();
        let manager = self.clone();
        let connection_id_clone = connection_id.clone();
        let connection_id_for_cleanup = connection_id.clone();

        // 处理接收到的消息
        let recv_task = {
            let manager = manager.clone();
            let connection_id = connection_id.clone();
            let monitor = monitor.clone();
            tokio::spawn(async move {
                while let Some(msg) = receiver.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            // 记录消息接收
                            info!(
                                "📨 WebSocket received message from connection_id: {}, length: {}",
                                connection_id,
                                text.len()
                            );
                            if let Some(ref monitor) = monitor {
                                monitor
                                    .record_message_received(&connection_id, text.len())
                                    .await;
                            }

                            if let Ok(ws_message) = serde_json::from_str::<WebSocketMessage>(&text)
                            {
                                // 完善消息 - 自动生成ID和timestamp
                                let complete_message = manager.complete_message(ws_message.clone());
                                info!("--------------------------------");
                                info!("ws_message: {:?}", complete_message);
                                info!("--------------------------------");
                                match complete_message.message_type {
                                    MessageType::Ping => {
                                        info!(
                                            "🏓 WebSocket Ping received from connection_id: {}",
                                            connection_id
                                        );
                                        manager.update_ping(&connection_id).await;
                                        let pong = WebSocketMessage {
                                            id: Some(Uuid::new_v4().to_string()),
                                            message_type: MessageType::Pong,
                                            data: serde_json::json!({"timestamp": chrono::Utc::now()}),
                                            timestamp: Some(chrono::Utc::now()),
                                        };
                                        manager.broadcast_message(pong).await;
                                    }
                                    MessageType::Command => {
                                        // 处理命令
                                        info!(
                                            "⚡ WebSocket Command received from connection_id: {}",
                                            connection_id
                                        );
                                        if let Some(ref handler) = command_handler {
                                            match serde_json::from_value::<
                                                crate::websocket::WebSocketCommand,
                                            >(
                                                ws_message.data.clone()
                                            ) {
                                                Ok(command) => {
                                                    let authenticated_user =
                                                        crate::websocket::auth::AuthenticatedUser {
                                                            user_id: user_id,
                                                            username: username.clone(),
                                                            email: "".to_string(), // 这里需要从数据库获取
                                                            name: username.clone(),
                                                            avatar_url: None,
                                                            current_workspace_id: user
                                                                .current_workspace_id, // 来自握手时的认证信息
                                                        };

                                                    // 标记此次命令是否会影响标签数据（在移动 command 之前计算）
                                                    let affects_labels = matches!(
                                                        &command,
                                                        crate::websocket::WebSocketCommand::CreateLabel { .. }
                                                            | crate::websocket::WebSocketCommand::UpdateLabel { .. }
                                                            | crate::websocket::WebSocketCommand::DeleteLabel { .. }
                                                            | crate::websocket::WebSocketCommand::BatchCreateLabels { .. }
                                                            | crate::websocket::WebSocketCommand::BatchUpdateLabels { .. }
                                                            | crate::websocket::WebSocketCommand::BatchDeleteLabels { .. }
                                                    );

                                                    // 标记此次命令是否会影响 workspace 数据
                                                    let affects_workspace = matches!(
                                                        &command,
                                                        crate::websocket::WebSocketCommand::UpdateWorkspace { .. }
                                                            | crate::websocket::WebSocketCommand::CreateWorkspace { .. }
                                                    );

                                                    let start_time = std::time::Instant::now();
                                                    let response = handler
                                                        .handle_command(
                                                            command,
                                                            &authenticated_user,
                                                        )
                                                        .await;
                                                    let response_time = start_time.elapsed();

                                                    // 记录命令处理监控
                                                    if let Some(ref monitor) = monitor {
                                                        monitor
                                                            .record_command_processed(
                                                                response_time,
                                                                response.success,
                                                            )
                                                            .await;
                                                    }

                                                    let response_message = WebSocketMessage {
                                                        id: Some(Uuid::new_v4().to_string()),
                                                        message_type: MessageType::CommandResponse,
                                                        data: serde_json::to_value(&response)
                                                            .unwrap(),
                                                        timestamp: Some(chrono::Utc::now()),
                                                    };
                                                    manager
                                                        .broadcast_message(response_message)
                                                        .await;

                                                    // 如果是影响标签数据的命令，追加一次 query_labels 的推送
                                                    if affects_labels {
                                                        // 构造一个 QueryLabels 命令（使用当前上下文工作区；filters 默认）
                                                        let refresh_cmd = crate::websocket::WebSocketCommand::QueryLabels {
                                                                filters: crate::websocket::commands::LabelFilters {
                                                                    workspace_id: None,
                                                                    level: None,
                                                                    name_pattern: None,
                                                                    color: None,
                                                                    created_after: None,
                                                                    created_before: None,
                                                                    limit: None,
                                                                    offset: None,
                                                                },
                                                                request_id: None,
                                                            };

                                                        let refresh_response = handler
                                                            .handle_command(
                                                                refresh_cmd,
                                                                &authenticated_user,
                                                            )
                                                            .await;

                                                        let refresh_message = WebSocketMessage {
                                                            id: Some(Uuid::new_v4().to_string()),
                                                            message_type:
                                                                MessageType::CommandResponse,
                                                            data: serde_json::to_value(
                                                                &refresh_response,
                                                            )
                                                            .unwrap(),
                                                            timestamp: Some(chrono::Utc::now()),
                                                        };
                                                        manager
                                                            .broadcast_message(refresh_message)
                                                            .await;
                                                    }

                                                    // 如果是影响 workspace 的命令，广播 get_current_workspace
                                                    if affects_workspace {
                                                        // 构造一个 GetCurrentWorkspace 命令
                                                        let refresh_cmd = crate::websocket::WebSocketCommand::GetCurrentWorkspace {
                                                            request_id: None,
                                                        };

                                                        let refresh_response = handler
                                                            .handle_command(
                                                                refresh_cmd,
                                                                &authenticated_user,
                                                            )
                                                            .await;

                                                        let refresh_message = WebSocketMessage {
                                                            id: Some(Uuid::new_v4().to_string()),
                                                            message_type:
                                                                MessageType::CommandResponse,
                                                            data: serde_json::to_value(
                                                                &refresh_response,
                                                            )
                                                            .unwrap(),
                                                            timestamp: Some(chrono::Utc::now()),
                                                        };

                                                        // 广播到同一workspace的所有用户
                                                        if let Some(workspace_id) =
                                                            authenticated_user.current_workspace_id
                                                        {
                                                            manager
                                                                .broadcast_to_workspace(
                                                                    workspace_id,
                                                                    refresh_message,
                                                                )
                                                                .await;
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    error!(
                                                        "❌ Failed to parse WebSocket command: {}, data: {}",
                                                        e, ws_message.data
                                                    );
                                                    // 发送错误响应
                                                    let error_response = crate::websocket::WebSocketCommandResponse::error(
                                                        "unknown",
                                                        "unknown",
                                                        None, // 解析失败时没有request_id
                                                        crate::websocket::WebSocketCommandError::system_error(&format!("Failed to parse command: {}", e)),
                                                    );

                                                    let error_message = WebSocketMessage {
                                                        id: Some(Uuid::new_v4().to_string()),
                                                        message_type: MessageType::CommandResponse,
                                                        data: serde_json::to_value(&error_response)
                                                            .unwrap(),
                                                        timestamp: Some(chrono::Utc::now()),
                                                    };
                                                    manager.broadcast_message(error_message).await;
                                                }
                                            }
                                        }
                                    }
                                    MessageType::Text => {
                                        // 广播文本消息
                                        info!(
                                            "💬 WebSocket Text message received from connection_id: {}",
                                            connection_id
                                        );
                                        manager.broadcast_message(complete_message).await;
                                    }
                                    _ => {
                                        // 处理其他类型的消息
                                        info!(
                                            "📋 WebSocket Other message type received from connection_id: {}, type: {:?}",
                                            connection_id, complete_message.message_type
                                        );
                                        manager.broadcast_message(complete_message).await;
                                    }
                                }
                            } else {
                                error!(
                                    "❌ WebSocket failed to parse message from connection_id: {}, text: {}",
                                    connection_id, text
                                );
                            }
                        }
                        Ok(Message::Close(_)) => {
                            info!(
                                "🔌 WebSocket connection closed for connection_id: {}",
                                connection_id
                            );
                            break;
                        }
                        Err(e) => {
                            error!(
                                "❌ WebSocket error for connection_id {}: {}",
                                connection_id, e
                            );
                            break;
                        }
                        _ => {}
                    }
                }
            })
        };

        // 处理广播消息
        let send_task = {
            let monitor = monitor.clone();
            tokio::spawn(async move {
                while let Ok(message) = rx.recv().await {
                    // 基于workspace广播 - 发送给同一workspace的所有用户
                    let should_send = true; // 所有广播消息都发送给当前连接

                    if should_send {
                        if let Ok(msg_text) = serde_json::to_string(&message) {
                            // 记录消息发送
                            info!(
                                "📤 WebSocket sending message to connection_id: {}, length: {}, type: {:?}",
                                connection_id_clone,
                                msg_text.len(),
                                message.message_type
                            );
                            if let Some(ref monitor) = monitor {
                                monitor
                                    .record_message_sent(&connection_id_clone, msg_text.len())
                                    .await;
                            }

                            if sender.send(Message::Text(msg_text)).await.is_err() {
                                break;
                            }
                        }
                    }
                }
            })
        };

        // 等待任务完成
        tokio::select! {
            _ = recv_task => {},
            _ = send_task => {},
        }

        // 清理连接
        self.remove_connection(&connection_id_for_cleanup).await;

        // 记录连接断开监控
        if let Some(ref monitor) = monitor {
            monitor
                .record_disconnection(&connection_id_for_cleanup)
                .await;
        }
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}
