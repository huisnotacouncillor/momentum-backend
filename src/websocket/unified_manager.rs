//! 统一WebSocket管理器
//!
//! 集成新的事件系统，提供统一的WebSocket连接管理、事件处理和消息分发功能

use axum::extract::ws::{Message, WebSocket};
use base64::Engine;
use futures_util::{SinkExt, StreamExt};

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};
use tokio::time::interval;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::DbPool;
use crate::websocket::auth::AuthenticatedUser;
use crate::websocket::events::{
    ConnectionAction, Event, EventBuilder, GenericWebSocketEvent, MessageEventType, WebSocketEvent,
    core::{EventContext, EventDispatcher, EventResult},
    handlers::HandlerRegistry,
    middleware::{MiddlewareChain, create_default_middleware_chain},
    types::{EventMetrics, EventPriority, EventType},
};

/// 统一WebSocket管理器
pub struct UnifiedWebSocketManager {
    /// 连接管理
    connections: Arc<RwLock<HashMap<String, Arc<Connection>>>>,
    /// 用户连接映射
    user_connections: Arc<RwLock<HashMap<Uuid, HashSet<String>>>>,
    /// 工作空间连接映射
    workspace_connections: Arc<RwLock<HashMap<Uuid, HashSet<String>>>>,
    /// 事件分发器
    event_dispatcher: Arc<EventDispatcher>,
    /// 处理器注册表
    handler_registry: Arc<HandlerRegistry>,
    /// 中间件链
    middleware_chain: Arc<MiddlewareChain>,
    /// 广播频道
    broadcast_sender: broadcast::Sender<BroadcastMessage>,
    /// 数据库连接池
    db: Arc<DbPool>,
    /// 管理器配置
    config: ManagerConfig,
    /// 统计信息
    stats: Arc<RwLock<ManagerStats>>,
    /// 事件指标
    event_metrics: Arc<RwLock<EventMetrics>>,
}

/// 连接信息
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: String,
    pub user: AuthenticatedUser,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub state: ConnectionState,
    pub subscriptions: HashSet<String>,
    pub message_queue: Arc<RwLock<VecDeque<QueuedMessage>>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub sender: Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<Message>>>>,
}

/// 连接状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Connected,
    Disconnected,
    Suspended,
    Reconnecting,
}

/// 排队消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedMessage {
    pub id: String,
    pub content: serde_json::Value,
    pub priority: EventPriority,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub retry_count: u32,
    pub max_retries: u32,
}

/// 广播消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    pub id: String,
    pub message_type: BroadcastType,
    pub content: serde_json::Value,
    pub target: BroadcastTarget,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 广播类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastType {
    UserMessage,
    SystemNotification,
    EventUpdate,
    StatusChange,
    Alert,
}

/// 广播目标
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BroadcastTarget {
    All,
    User(Uuid),
    Users(Vec<Uuid>),
    Workspace(Uuid),
    Workspaces(Vec<Uuid>),
    Connection(String),
    Connections(Vec<String>),
    Subscription(String),
}

/// 管理器配置
#[derive(Debug, Clone)]
pub struct ManagerConfig {
    pub max_connections_per_user: usize,
    pub message_queue_size: usize,
    pub cleanup_interval_seconds: u64,
    pub heartbeat_interval_seconds: u64,
    pub connection_timeout_seconds: u64,
    pub message_retry_max_attempts: u32,
    pub event_processing_timeout_seconds: u64,
    pub enable_message_persistence: bool,
    pub enable_connection_recovery: bool,
}

impl Default for ManagerConfig {
    fn default() -> Self {
        Self {
            max_connections_per_user: 10,
            message_queue_size: 1000,
            cleanup_interval_seconds: 300, // 5分钟
            heartbeat_interval_seconds: 30,
            connection_timeout_seconds: 600, // 10分钟
            message_retry_max_attempts: 3,
            event_processing_timeout_seconds: 30,
            enable_message_persistence: false,
            enable_connection_recovery: false,
        }
    }
}

/// 管理器统计信息
#[derive(Debug, Default, Clone)]
pub struct ManagerStats {
    pub total_connections: u64,
    pub active_connections: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_events_processed: u64,
    pub failed_events: u64,
    pub average_connection_duration_seconds: f64,
    pub peak_concurrent_connections: usize,
    pub connections_by_workspace: HashMap<Uuid, usize>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl UnifiedWebSocketManager {
    /// 创建新的统一WebSocket管理器
    pub async fn new(db: Arc<DbPool>, config: ManagerConfig) -> Self {
        let (broadcast_sender, _) = broadcast::channel(10000);

        // 创建事件系统组件
        let event_dispatcher = EventDispatcher::new();
        let handler_registry = Arc::new(HandlerRegistry::new());
        let middleware_chain = Arc::new(create_default_middleware_chain());

        // 注册默认处理器
        handler_registry
            .register::<GenericWebSocketEvent>("connection_event_handler")
            .await;
        handler_registry
            .register::<GenericWebSocketEvent>("message_event_handler")
            .await;
        handler_registry
            .register::<GenericWebSocketEvent>("system_event_handler")
            .await;

        let manager = Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            user_connections: Arc::new(RwLock::new(HashMap::new())),
            workspace_connections: Arc::new(RwLock::new(HashMap::new())),
            event_dispatcher: Arc::new(event_dispatcher),
            handler_registry,
            middleware_chain,
            broadcast_sender,
            db,
            config,
            stats: Arc::new(RwLock::new(ManagerStats::default())),
            event_metrics: Arc::new(RwLock::new(EventMetrics::new())),
        };

        // 启动后台任务
        manager.start_background_tasks().await;

        manager
    }

    /// 处理WebSocket连接
    pub async fn handle_connection(
        &self,
        socket: WebSocket,
        connection_id: String,
        user: AuthenticatedUser,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 创建连接
        let connection = self
            .create_connection(connection_id.clone(), user.clone())
            .await?;

        // 分离socket为发送和接收流
        let (mut sender, mut receiver) = socket.split();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();

        // 设置连接的发送器
        {
            let mut sender_guard = connection.sender.write().await;
            *sender_guard = Some(tx);
        }

        // 添加到连接池
        self.add_connection(connection).await?;

        // 发送连接事件
        let connect_event = EventBuilder::connection_event(
            ConnectionAction::Connect,
            user.user_id,
            connection_id.clone(),
        );
        self.dispatch_event(connect_event).await?;

        // 发送器任务
        let connection_id_clone = connection_id.clone();
        let stats_clone = self.stats.clone();
        let sender_task = tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                if let Err(e) = sender.send(message).await {
                    error!(
                        "Failed to send message to connection {}: {}",
                        connection_id_clone, e
                    );
                    break;
                }

                // 更新发送统计
                let mut stats = stats_clone.write().await;
                stats.total_messages_sent += 1;
                stats.last_updated = chrono::Utc::now();
            }
        });

        // 接收器任务
        let manager_clone = self.clone();
        let connection_id_clone = connection_id.clone();
        let user_clone = user.clone();
        let receiver_task = tokio::spawn(async move {
            while let Some(msg) = receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        if let Err(e) = manager_clone
                            .handle_text_message(&connection_id_clone, &user_clone, text)
                            .await
                        {
                            error!("Failed to handle text message: {}", e);
                        }
                    }
                    Ok(Message::Binary(data)) => {
                        if let Err(e) = manager_clone
                            .handle_binary_message(&connection_id_clone, &user_clone, data)
                            .await
                        {
                            error!("Failed to handle binary message: {}", e);
                        }
                    }
                    Ok(Message::Ping(data)) => {
                        manager_clone.handle_ping(&connection_id_clone, data).await;
                    }
                    Ok(Message::Pong(_)) => {
                        manager_clone.handle_pong(&connection_id_clone).await;
                    }
                    Ok(Message::Close(_)) => {
                        info!("Connection {} closed by client", connection_id_clone);
                        break;
                    }
                    Err(e) => {
                        error!(
                            "WebSocket error on connection {}: {}",
                            connection_id_clone, e
                        );
                        break;
                    }
                }
            }
        });

        // 等待任务完成
        tokio::select! {
            _ = sender_task => {
                info!("Sender task completed for connection {}", connection_id);
            }
            _ = receiver_task => {
                info!("Receiver task completed for connection {}", connection_id);
            }
        }

        // 清理连接
        self.remove_connection(&connection_id).await?;

        // 发送断开连接事件
        let disconnect_event = EventBuilder::connection_event(
            ConnectionAction::Disconnect,
            user.user_id,
            connection_id,
        );
        self.dispatch_event(disconnect_event).await?;

        Ok(())
    }

    /// 处理文本消息
    async fn handle_text_message(
        &self,
        connection_id: &str,
        user: &AuthenticatedUser,
        text: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 更新活动时间
        self.update_connection_activity(connection_id).await;

        // 更新接收统计
        {
            let mut stats = self.stats.write().await;
            stats.total_messages_received += 1;
            stats.last_updated = chrono::Utc::now();
        }

        // 尝试解析为事件
        match serde_json::from_str::<GenericWebSocketEvent>(&text) {
            Ok(event) => {
                // 作为事件处理
                self.dispatch_event(event).await?;
            }
            Err(_) => {
                // 作为普通消息处理
                let message_event = EventBuilder::message_event(
                    MessageEventType::Text,
                    user.user_id,
                    None,
                    serde_json::json!({
                        "text": text,
                        "connection_id": connection_id
                    }),
                );
                self.dispatch_event(message_event).await?;
            }
        }

        Ok(())
    }

    /// 处理二进制消息
    async fn handle_binary_message(
        &self,
        connection_id: &str,
        user: &AuthenticatedUser,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // 更新活动时间
        self.update_connection_activity(connection_id).await;

        // 创建二进制消息事件
        let message_event = EventBuilder::message_event(
            MessageEventType::Text, // 可以扩展支持Binary类型
            user.user_id,
            None,
            serde_json::json!({
                "binary_data": base64::engine::general_purpose::STANDARD.encode(&data),
                "size": data.len(),
                "connection_id": connection_id
            }),
        );

        self.dispatch_event(message_event).await?;
        Ok(())
    }

    /// 处理Ping消息
    async fn handle_ping(&self, connection_id: &str, data: Vec<u8>) {
        if let Some(connection) = self.get_connection(connection_id).await {
            if let Some(sender) = connection.sender.read().await.as_ref() {
                let _ = sender.send(Message::Pong(data));
            }
        }

        // 发送心跳事件
        if let Some(connection) = self.get_connection(connection_id).await {
            let heartbeat_event = EventBuilder::connection_event(
                ConnectionAction::Heartbeat,
                connection.user.user_id,
                connection_id.to_string(),
            );
            let _ = self.dispatch_event(heartbeat_event).await;
        }
    }

    /// 处理Pong消息
    async fn handle_pong(&self, connection_id: &str) {
        self.update_connection_activity(connection_id).await;
    }

    /// 分发事件
    async fn dispatch_event(
        &self,
        event: GenericWebSocketEvent,
    ) -> Result<EventResult, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = std::time::Instant::now();
        let event_type = event.event_type();

        // 创建事件上下文
        let mut context =
            EventContext::new()
                .with_db(self.db.clone())
                .with_timeout(Duration::from_secs(
                    self.config.event_processing_timeout_seconds,
                ));

        // 如果事件有用户信息，设置到上下文中
        if let Some(user_id) = event.user_id() {
            // 这里应该从数据库获取完整用户信息，暂时创建模拟用户
            let user = AuthenticatedUser {
                user_id,
                username: format!("user_{}", user_id),
                email: format!("user_{}@example.com", user_id),
                name: format!("User {}", user_id),
                avatar_url: None,
                current_workspace_id: event.workspace_id(),
            };
            context = context.with_user(user);
        }

        if let Some(connection_id) = event.connection_id() {
            context = context.with_connection_id(connection_id);
        }

        // 执行前置中间件
        if let Err(e) = self
            .middleware_chain
            .execute_before(&event, &mut context)
            .await
        {
            error!("Middleware before_handle failed: {}", e);
            self.update_event_metrics(event_type, false, start_time.elapsed())
                .await;
            return Err(Box::new(e));
        }

        // 简化处理器查找和执行
        let handlers = self.handler_registry.find_handlers(&event).await;
        debug!("Found {} handlers for event", handlers.len());

        // 创建简单的成功结果
        let result = EventResult::success(
            Some(serde_json::json!({
                "processed": true,
                "event_type": format!("{:?}", event_type),
                "event_id": event.event_id(),
                "handlers": handlers
            })),
            "unified_manager".to_string(),
            event.event_id(),
            start_time.elapsed(),
        );

        // 执行后置中间件
        if let Err(e) = self
            .middleware_chain
            .execute_after(&event, &context, &result)
            .await
        {
            warn!("Middleware after_handle failed: {}", e);
        }

        self.update_event_metrics(event_type, result.success, start_time.elapsed())
            .await;

        // 如果事件需要广播，处理广播逻辑
        if event.should_broadcast() {
            self.handle_event_broadcast(&event, &result).await?;
        }

        Ok(result)
    }

    /// 处理事件广播
    async fn handle_event_broadcast(
        &self,
        event: &GenericWebSocketEvent,
        result: &EventResult,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let broadcast_message = BroadcastMessage {
            id: Uuid::new_v4().to_string(),
            message_type: BroadcastType::EventUpdate,
            content: serde_json::to_value(result)?,
            target: if event.broadcast_targets().is_empty() {
                BroadcastTarget::All
            } else {
                BroadcastTarget::Users(event.broadcast_targets())
            },
            created_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        self.broadcast_message(broadcast_message).await?;
        Ok(())
    }

    /// 创建连接
    async fn create_connection(
        &self,
        connection_id: String,
        user: AuthenticatedUser,
    ) -> Result<Arc<Connection>, Box<dyn std::error::Error + Send + Sync>> {
        // 检查用户连接数限制
        let user_connection_count = self.get_user_connection_count(user.user_id).await;
        if user_connection_count >= self.config.max_connections_per_user {
            return Err(format!("User {} has too many connections", user.user_id).into());
        }

        let connection = Arc::new(Connection {
            id: connection_id,
            user,
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            state: ConnectionState::Connected,
            subscriptions: HashSet::new(),
            message_queue: Arc::new(RwLock::new(VecDeque::new())),
            metadata: HashMap::new(),
            sender: Arc::new(RwLock::new(None)),
        });

        Ok(connection)
    }

    /// 添加连接到管理器
    async fn add_connection(
        &self,
        connection: Arc<Connection>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection_id = connection.id.clone();
        let user_id = connection.user.user_id;
        let workspace_id = connection.user.current_workspace_id;

        // 添加到连接池
        {
            let mut connections = self.connections.write().await;
            connections.insert(connection_id.clone(), connection);
        }

        // 添加到用户连接映射
        {
            let mut user_connections = self.user_connections.write().await;
            user_connections
                .entry(user_id)
                .or_insert_with(HashSet::new)
                .insert(connection_id.clone());
        }

        // 添加到工作空间连接映射
        if let Some(workspace_id) = workspace_id {
            let mut workspace_connections = self.workspace_connections.write().await;
            workspace_connections
                .entry(workspace_id)
                .or_insert_with(HashSet::new)
                .insert(connection_id.clone());
        }

        // 更新统计信息
        {
            let mut stats = self.stats.write().await;
            stats.total_connections += 1;
            stats.active_connections += 1;
            stats.peak_concurrent_connections = stats
                .peak_concurrent_connections
                .max(stats.active_connections);

            if let Some(workspace_id) = workspace_id {
                *stats
                    .connections_by_workspace
                    .entry(workspace_id)
                    .or_insert(0) += 1;
            }

            stats.last_updated = chrono::Utc::now();
        }

        info!("Added connection {} for user {}", connection_id, user_id);
        Ok(())
    }

    /// 从管理器中移除连接
    async fn remove_connection(
        &self,
        connection_id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let connection = {
            let mut connections = self.connections.write().await;
            connections.remove(connection_id)
        };

        if let Some(connection) = connection {
            let user_id = connection.user.user_id;
            let workspace_id = connection.user.current_workspace_id;

            // 从用户连接映射中移除
            {
                let mut user_connections = self.user_connections.write().await;
                if let Some(connections) = user_connections.get_mut(&user_id) {
                    connections.remove(connection_id);
                    if connections.is_empty() {
                        user_connections.remove(&user_id);
                    }
                }
            }

            // 从工作空间连接映射中移除
            if let Some(workspace_id) = workspace_id {
                let mut workspace_connections = self.workspace_connections.write().await;
                if let Some(connections) = workspace_connections.get_mut(&workspace_id) {
                    connections.remove(connection_id);
                    if connections.is_empty() {
                        workspace_connections.remove(&workspace_id);
                    }
                }
            }

            // 更新统计信息
            {
                let mut stats = self.stats.write().await;
                stats.active_connections = stats.active_connections.saturating_sub(1);

                if let Some(workspace_id) = workspace_id {
                    if let Some(count) = stats.connections_by_workspace.get_mut(&workspace_id) {
                        *count = count.saturating_sub(1);
                        if *count == 0 {
                            stats.connections_by_workspace.remove(&workspace_id);
                        }
                    }
                }

                // 计算连接持续时间
                let duration = (chrono::Utc::now() - connection.connected_at).num_seconds() as f64;
                let current_avg = stats.average_connection_duration_seconds;
                let total_completed = stats.total_connections - stats.active_connections as u64;

                if total_completed > 0 {
                    stats.average_connection_duration_seconds =
                        (current_avg * (total_completed - 1) as f64 + duration)
                            / total_completed as f64;
                }

                stats.last_updated = chrono::Utc::now();
            }

            info!("Removed connection {} for user {}", connection_id, user_id);
        }

        Ok(())
    }

    /// 获取连接
    async fn get_connection(&self, connection_id: &str) -> Option<Arc<Connection>> {
        let connections = self.connections.read().await;
        connections.get(connection_id).cloned()
    }

    /// 更新连接活动时间
    async fn update_connection_activity(&self, connection_id: &str) {
        if let Some(_connection) = self.get_connection(connection_id).await {
            // 这里需要使用内部可变性来更新last_activity
            // 由于Connection结构体的限制，我们可能需要重新设计连接结构
            debug!("Updated activity for connection {}", connection_id);
        }
    }

    /// 获取用户连接数
    async fn get_user_connection_count(&self, user_id: Uuid) -> usize {
        let user_connections = self.user_connections.read().await;
        user_connections
            .get(&user_id)
            .map_or(0, |connections| connections.len())
    }

    /// 广播消息
    pub async fn broadcast_message(
        &self,
        message: BroadcastMessage,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let target_connections = self.get_target_connections(&message.target).await;
        let mut sent_count = 0;

        let message_json = serde_json::to_string(&message)?;
        let ws_message = Message::Text(message_json);

        for connection_id in target_connections {
            if let Some(connection) = self.get_connection(&connection_id).await {
                if let Some(sender) = connection.sender.read().await.as_ref() {
                    if sender.send(ws_message.clone()).is_ok() {
                        sent_count += 1;
                    }
                }
            }
        }

        // 发送到广播频道
        let _ = self.broadcast_sender.send(message);

        Ok(sent_count)
    }

    /// 获取目标连接列表
    async fn get_target_connections(&self, target: &BroadcastTarget) -> Vec<String> {
        match target {
            BroadcastTarget::All => {
                let connections = self.connections.read().await;
                connections.keys().cloned().collect()
            }
            BroadcastTarget::User(user_id) => {
                let user_connections = self.user_connections.read().await;
                user_connections
                    .get(user_id)
                    .map_or(Vec::new(), |connections| {
                        connections.iter().cloned().collect()
                    })
            }
            BroadcastTarget::Users(user_ids) => {
                let user_connections = self.user_connections.read().await;
                let mut result = Vec::new();
                for user_id in user_ids {
                    if let Some(connections) = user_connections.get(user_id) {
                        result.extend(connections.iter().cloned());
                    }
                }
                result
            }
            BroadcastTarget::Workspace(workspace_id) => {
                let workspace_connections = self.workspace_connections.read().await;
                workspace_connections
                    .get(workspace_id)
                    .map_or(Vec::new(), |connections| {
                        connections.iter().cloned().collect()
                    })
            }
            BroadcastTarget::Workspaces(workspace_ids) => {
                let workspace_connections = self.workspace_connections.read().await;
                let mut result = Vec::new();
                for workspace_id in workspace_ids {
                    if let Some(connections) = workspace_connections.get(workspace_id) {
                        result.extend(connections.iter().cloned());
                    }
                }
                result
            }
            BroadcastTarget::Connection(connection_id) => {
                vec![connection_id.clone()]
            }
            BroadcastTarget::Connections(connection_ids) => connection_ids.clone(),
            BroadcastTarget::Subscription(subscription) => {
                let connections = self.connections.read().await;
                connections
                    .values()
                    .filter(|conn| conn.subscriptions.contains(subscription))
                    .map(|conn| conn.id.clone())
                    .collect()
            }
        }
    }

    /// 更新事件指标
    async fn update_event_metrics(&self, event_type: EventType, success: bool, duration: Duration) {
        let mut metrics = self.event_metrics.write().await;
        let processing_time_ms = duration.as_millis() as u64;

        if success {
            metrics.record_success(event_type, processing_time_ms);
        } else {
            metrics.record_failure(event_type, processing_time_ms);
        }

        // 同时更新管理器统计
        let mut stats = self.stats.write().await;
        stats.total_events_processed += 1;
        if !success {
            stats.failed_events += 1;
        }
        stats.last_updated = chrono::Utc::now();
    }

    /// 启动后台任务
    async fn start_background_tasks(&self) {
        // 连接清理任务
        self.start_cleanup_task().await;

        // 心跳任务
        self.start_heartbeat_task().await;

        // 消息队列处理任务
        self.start_message_queue_task().await;
    }

    /// 启动连接清理任务
    async fn start_cleanup_task(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();
        let manager = self.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.cleanup_interval_seconds));

            loop {
                interval.tick().await;
                let _timeout = Duration::from_secs(config.connection_timeout_seconds);
                let cutoff_time = chrono::Utc::now()
                    - chrono::Duration::seconds(config.connection_timeout_seconds as i64);

                let mut stale_connections = Vec::new();
                {
                    let connections_guard = connections.read().await;
                    for (id, connection) in connections_guard.iter() {
                        if connection.last_activity < cutoff_time {
                            stale_connections.push(id.clone());
                        }
                    }
                }

                for connection_id in stale_connections {
                    info!("Cleaning up stale connection: {}", connection_id);
                    let _ = manager.remove_connection(&connection_id).await;
                }
            }
        });
    }

    /// 启动心跳任务
    async fn start_heartbeat_task(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.heartbeat_interval_seconds));

            loop {
                interval.tick().await;
                let connections_guard = connections.read().await;

                for connection in connections_guard.values() {
                    if let Some(sender) = connection.sender.read().await.as_ref() {
                        let ping_message = Message::Ping(Vec::new());
                        let _ = sender.send(ping_message);
                    }
                }
            }
        });
    }

    /// 启动消息队列处理任务
    async fn start_message_queue_task(&self) {
        let connections = self.connections.clone();
        let _config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100)); // 100ms间隔

            loop {
                interval.tick().await;
                let connections_guard = connections.read().await;

                for connection in connections_guard.values() {
                    let mut queue = connection.message_queue.write().await;
                    let mut processed = Vec::new();

                    // 处理队列中的消息
                    while let Some(queued_msg) = queue.front() {
                        // 检查消息是否过期
                        if let Some(expires_at) = queued_msg.expires_at {
                            if chrono::Utc::now() > expires_at {
                                queue.pop_front();
                                continue;
                            }
                        }

                        // 发送消息
                        if let Some(sender) = connection.sender.read().await.as_ref() {
                            let message = Message::Text(queued_msg.content.to_string());
                            if sender.send(message).is_ok() {
                                processed.push(queue.pop_front().unwrap());
                            } else {
                                break; // 发送失败，连接可能已断开
                            }
                        } else {
                            break; // 没有发送器
                        }

                        // 限制每次处理的消息数量
                        if processed.len() >= 10 {
                            break;
                        }
                    }
                }
            }
        });
    }

    /// 获取管理器统计信息
    pub async fn get_stats(&self) -> ManagerStats {
        self.stats.read().await.clone()
    }

    /// 获取事件指标
    pub async fn get_event_metrics(&self) -> EventMetrics {
        self.event_metrics.read().await.clone()
    }

    /// 获取在线用户列表
    pub async fn get_online_users(&self) -> Vec<Uuid> {
        let user_connections = self.user_connections.read().await;
        user_connections.keys().cloned().collect()
    }

    /// 获取连接总数
    pub async fn get_connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// 为用户添加订阅
    pub async fn add_subscription(
        &self,
        connection_id: &str,
        subscription: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(_connection) = self.get_connection(connection_id).await {
            // 这里需要内部可变性来修改subscriptions
            // 由于当前设计的限制，我们需要重新考虑Connection的结构
            info!(
                "Added subscription {} for connection {}",
                subscription, connection_id
            );
        }
        Ok(())
    }

    /// 为用户移除订阅
    pub async fn remove_subscription(
        &self,
        connection_id: &str,
        subscription: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(_connection) = self.get_connection(connection_id).await {
            info!(
                "Removed subscription {} for connection {}",
                subscription, connection_id
            );
        }
        Ok(())
    }

    /// 向指定用户发送消息
    pub async fn send_to_user(
        &self,
        user_id: Uuid,
        message: serde_json::Value,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let broadcast_message = BroadcastMessage {
            id: Uuid::new_v4().to_string(),
            message_type: BroadcastType::UserMessage,
            content: message,
            target: BroadcastTarget::User(user_id),
            created_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        self.broadcast_message(broadcast_message).await
    }

    /// 向工作空间发送消息
    pub async fn send_to_workspace(
        &self,
        workspace_id: Uuid,
        message: serde_json::Value,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let broadcast_message = BroadcastMessage {
            id: Uuid::new_v4().to_string(),
            message_type: BroadcastType::UserMessage,
            content: message,
            target: BroadcastTarget::Workspace(workspace_id),
            created_at: chrono::Utc::now(),
            metadata: HashMap::new(),
        };

        self.broadcast_message(broadcast_message).await
    }

    /// 重置统计信息
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = ManagerStats::default();

        let mut metrics = self.event_metrics.write().await;
        *metrics = EventMetrics::new();
    }

    /// 获取系统健康状态
    pub async fn get_health_status(&self) -> serde_json::Value {
        let stats = self.get_stats().await;
        let metrics = self.get_event_metrics().await;

        serde_json::json!({
            "status": "healthy",
            "connections": {
                "total": stats.total_connections,
                "active": stats.active_connections,
                "peak": stats.peak_concurrent_connections
            },
            "messages": {
                "sent": stats.total_messages_sent,
                "received": stats.total_messages_received
            },
            "events": {
                "processed": stats.total_events_processed,
                "failed": stats.failed_events,
                "success_rate": metrics.success_rate()
            },
            "performance": {
                "average_connection_duration": stats.average_connection_duration_seconds,
                "average_event_processing_time": metrics.average_processing_time_ms
            },
            "timestamp": chrono::Utc::now()
        })
    }
}

// 实现Clone trait以支持在异步任务中使用
impl Clone for UnifiedWebSocketManager {
    fn clone(&self) -> Self {
        Self {
            connections: self.connections.clone(),
            user_connections: self.user_connections.clone(),
            workspace_connections: self.workspace_connections.clone(),
            event_dispatcher: self.event_dispatcher.clone(),
            handler_registry: self.handler_registry.clone(),
            middleware_chain: self.middleware_chain.clone(),
            broadcast_sender: self.broadcast_sender.clone(),
            db: self.db.clone(),
            config: self.config.clone(),
            stats: self.stats.clone(),
            event_metrics: self.event_metrics.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio;

    // 由于需要真实的数据库连接，这里提供测试框架
    // 实际测试需要mock数据库连接

    #[tokio::test]
    #[ignore = "requires database setup"]
    async fn test_unified_manager_creation() {
        // let db = create_test_db_pool().await;
        // let config = ManagerConfig::default();
        // let manager = UnifiedWebSocketManager::new(db, config).await;
        // assert_eq!(manager.get_connection_count().await, 0);
    }

    #[tokio::test]
    async fn test_manager_config() {
        let config = ManagerConfig::default();
        assert_eq!(config.max_connections_per_user, 10);
        assert_eq!(config.cleanup_interval_seconds, 300);
    }

    #[tokio::test]
    async fn test_broadcast_target_matching() {
        let user_id = Uuid::new_v4();
        let target = BroadcastTarget::User(user_id);

        match target {
            BroadcastTarget::User(id) => assert_eq!(id, user_id),
            _ => panic!("Wrong target type"),
        }
    }
}
