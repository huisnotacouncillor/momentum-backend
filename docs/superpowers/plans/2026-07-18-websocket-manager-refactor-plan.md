# WebSocketManager 重构实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 将 1153 行 `manager.rs` 拆分为 connection.rs、broadcast.rs、recovery.rs、offline_queue.rs、subscription.rs 五个子模块，通过 mod.rs Facade 统一暴露

**架构：** Facade 模式，WebSocketManager 保留原 API 签名，内部委托给五个子模块。subscription/ 目录内容合并到 manager/subscription.rs。

**技术栈：** Rust (tokio, broadcast channel, RwLock)

---

## 文件变更概览

| 操作 | 文件 |
|------|------|
| 创建 | `websocket/manager/mod.rs` — Facade |
| 创建 | `websocket/manager/connection.rs` — 连接生命周期 |
| 创建 | `websocket/manager/broadcast.rs` — 广播 + 精准发送 |
| 创建 | `websocket/manager/recovery.rs` — 断连恢复 |
| 创建 | `websocket/manager/offline_queue.rs` — 离线消息队列 |
| 创建 | `websocket/manager/subscription.rs` — 从 subscription/ 迁移 |
| 创建 | `websocket/manager/state.rs` — 共享类型（ConnectedUser, MessageType 等） |
| 删除 | `websocket/manager.rs` — 原文件 |
| 删除 | `websocket/subscription/` — 目录整体迁移后删除 |

---

## 共享类型定义 (state.rs)

**文件：** `momentum_api/src/websocket/manager/state.rs`（新建）

包含所有子模块共用的类型，从原 manager.rs 顶部提取：

```rust
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
use chrono::{DateTime, Utc};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecoveryInfo {
    pub user_id: Uuid,
    pub recovery_token: String,
    pub expires_at: DateTime<Utc>,
    pub pending_messages: VecDeque<WebSocketMessage>,
}
```

---

## 任务 1：创建 manager/ 目录结构

**文件：** 无（仅创建目录）

- [ ] **步骤 1：创建目录**

```bash
mkdir -p momentum_api/src/websocket/manager
```

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager
git commit -m "refactor(websocket): create manager/ directory structure"
```

---

## 任务 2：创建 state.rs（共享类型）

**文件：**
- 创建：`momentum_api/src/websocket/manager/state.rs`
- 参考：`momentum_api/src/websocket/manager.rs:1-70`（types 部分）

- [ ] **步骤 1：创建 state.rs**

从 `manager.rs:1-70` 提取所有共享类型到新文件。

```rust
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionRecoveryInfo {
    pub user_id: Uuid,
    pub recovery_token: String,
    pub expires_at: DateTime<Utc>,
    pub pending_messages: VecDeque<WebSocketMessage>,
}
```

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "(error|warning)" | head -20
```

预期：会有"未使用的模块"警告，但无 error

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/state.rs
git commit -m "refactor(websocket): extract shared types to state.rs"
```

---

## 任务 3：创建 connection.rs

**文件：**
- 创建：`momentum_api/src/websocket/manager/connection.rs`
- 参考：`momentum_api/src/websocket/manager.rs:88-340`（add_connection, remove_connection, suspend_connection, resume_connection, get_connection, update_ping, get_online_users, get_connection_count, cleanup_stale_connections, handle_socket 前半部分）

- [ ] **步骤 1：创建 connection.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use uuid::Uuid;

use super::state::{ConnectedUser, ConnectionState, WebSocketMessage};

pub struct ConnectionManager {
    connections: Arc<RwLock<HashMap<String, ConnectedUser>>>,
    max_queue_size: usize,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            max_queue_size: 100,
        }
    }

    pub async fn add_connection(
        &self,
        connection_id: String,
        user: ConnectedUser,
    ) {
        let mut connections = self.connections.write().await;
        connections.insert(connection_id.clone(), user.clone());
        info!(
            "🔌 WebSocket User {} connected with connection ID {}",
            user.username, connection_id
        );
    }

    pub async fn remove_connection(&self, connection_id: &str) -> Option<ConnectedUser> {
        let mut connections = self.connections.write().await;
        connections.remove(connection_id)
    }

    pub async fn suspend_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.state = ConnectionState::Suspended;
            info!("⏸️ WebSocket Suspended connection for user {}", user.username);
        }
    }

    pub async fn resume_connection(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.state = ConnectionState::Connected;
            info!("▶️ WebSocket Resumed connection for user {}", user.username);
        }
    }

    pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectedUser> {
        let connections = self.connections.read().await;
        connections.get(connection_id).cloned()
    }

    pub async fn update_ping(&self, connection_id: &str) {
        let mut connections = self.connections.write().await;
        if let Some(user) = connections.get_mut(connection_id) {
            user.last_ping = chrono::Utc::now();
        }
    }

    pub async fn get_online_users(&self) -> Vec<ConnectedUser> {
        let connections = self.connections.read().await;
        connections.values().cloned().collect()
    }

    pub async fn get_connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    pub async fn get_user_mut(&self, user_id: Uuid) -> Option<ConnectedUser> {
        let mut connections = self.connections.write().await;
        // 查找用户的任一连接
        for (id, user) in connections.iter_mut() {
            if user.user_id == user_id {
                return Some(user.clone());
            }
        }
        None
    }

    pub async fn get_connections_in_workspace(&self, workspace_id: Uuid) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, user)| user.current_workspace_id == Some(workspace_id))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub async fn get_connections_by_user(&self, user_id: Uuid) -> Vec<String> {
        let connections = self.connections.read().await;
        connections
            .iter()
            .filter(|(_, user)| user.user_id == user_id)
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub async fn cleanup_stale_connections(&self, timeout_minutes: i64) -> usize {
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(timeout_minutes);
        let mut connections = self.connections.write().await;
        let stale: Vec<_> = connections
            .iter()
            .filter(|(_, user)| user.last_ping < cutoff)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &stale {
            connections.remove(id);
        }
        stale.len()
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -10
```

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/connection.rs
git commit -m "refactor(websocket): extract ConnectionManager to connection.rs"
```

---

## 任务 4：创建 broadcast.rs

**文件：**
- 创建：`momentum_api/src/websocket/manager/broadcast.rs`
- 参考：`momentum_api/src/websocket/manager.rs:340-430`（broadcast_message, broadcast_to_workspace, send_to_user, get_broadcast_receiver, direct_send, broadcast_issue_event, broadcast_issue_event_to_workspace）

**注意：** broadcast.rs 需要依赖 connection.rs（查询工作区连接）。在 Facade 阶段通过 Arc 共享引用解决。

- [ ] **步骤 1：创建 broadcast.rs 框架**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info, warn};
use uuid::Uuid;

use super::state::{MessageType, WebSocketMessage};
use super::issue_events::IssueEvent;

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
```

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -10
```

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/broadcast.rs
git commit -m "refactor(websocket): extract BroadcastManager to broadcast.rs"
```

---

## 任务 5：创建 recovery.rs

**文件：**
- 创建：`momentum_api/src/websocket/manager/recovery.rs`
- 参考：`momentum_api/src/websocket/manager.rs:190-250`（create_recovery_info, recover_connection）

- [ ] **步骤 1：创建 recovery.rs**

```rust
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;
use uuid::Uuid;

use super::state::{ConnectionRecoveryInfo, ConnectedUser, WebSocketMessage};

pub struct RecoveryManager {
    recovery_info: Arc<RwLock<HashMap<Uuid, ConnectionRecoveryInfo>>>,
    recovery_token_ttl: Duration,
}

impl RecoveryManager {
    pub fn new() -> Self {
        Self {
            recovery_info: Arc::new(RwLock::new(HashMap::new())),
            recovery_token_ttl: Duration::from_secs(300),
        }
    }

    pub async fn create_recovery_info(&self, user: &ConnectedUser) {
        let recovery_token = Uuid::new_v4().to_string();
        let expires_at =
            chrono::Utc::now() + chrono::Duration::from_std(self.recovery_token_ttl).unwrap();

        let recovery_info = ConnectionRecoveryInfo {
            user_id: user.user_id,
            recovery_token: recovery_token.clone(),
            expires_at,
            pending_messages: user.message_queue.clone(),
        };

        let mut recovery_map = self.recovery_info.write().await;
        recovery_map.insert(user.user_id, recovery_info);

        info!(
            "🔄 WebSocket Created recovery info for user {} with token {}",
            user.username, recovery_token
        );
    }

    pub async fn recover_connection(
        &self,
        user_id: Uuid,
        recovery_token: &str,
    ) -> Option<ConnectedUser> {
        let pending_messages = {
            let recovery_map = self.recovery_info.read().await;
            let info = recovery_map.get(&user_id)?;
            if info.recovery_token != recovery_token
                || info.expires_at <= chrono::Utc::now()
            {
                return None;
            }
            info.pending_messages.clone()
        };

        info!("🔄 WebSocket Recovered connection for user {}", user_id);
        Some(ConnectedUser {
            message_queue: pending_messages,
            ..Default::default()
        })
    }
}

impl Default for RecoveryManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -10
```

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/recovery.rs
git commit -m "refactor(websocket): extract RecoveryManager to recovery.rs"
```

---

## 任务 6：创建 offline_queue.rs

**文件：**
- 创建：`momentum_api/src/websocket/manager/offline_queue.rs`
- 参考：`momentum_api/src/websocket/manager.rs:270-310`（add_offline_message, get_offline_messages）

**设计**：OfflineQueueManager 直接持有 `Arc<ConnectionManager>`，通过它访问 ConnectedUser.message_queue。

- [ ] **步骤 1：创建 offline_queue.rs**

```rust
use std::collections::VecDeque;
use std::sync::Arc;
use uuid::Uuid;

use super::connection::ConnectionManager;
use super::state::{ConnectedUser, WebSocketMessage};

pub struct OfflineQueueManager {
    conn: Arc<ConnectionManager>,
    max_queue_size: usize,
}

impl OfflineQueueManager {
    pub fn new(conn: Arc<ConnectionManager>) -> Self {
        Self { conn, max_queue_size: 100 }
    }

    pub async fn add_offline_message(&self, user_id: Uuid, message: WebSocketMessage) {
        if let Some(mut user) = self.conn.get_user_mut(user_id).await {
            user.message_queue.push_back(message);
            if user.message_queue.len() > self.max_queue_size {
                user.message_queue.pop_front();
            }
        }
    }

    pub async fn get_offline_messages(&self, user_id: Uuid) -> VecDeque<WebSocketMessage> {
        if let Some(mut user) = self.conn.get_user_mut(user_id).await {
            let messages = user.message_queue.clone();
            user.message_queue.clear();
            messages
        } else {
            VecDeque::new()
        }
    }
}
```

**注意**：ConnectionManager 需要新增 `get_user_mut(user_id)` 方法来支持修改。

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -10
```

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/offline_queue.rs
git commit -m "refactor(websocket): extract OfflineQueueManager to offline_queue.rs"
```

---

## 任务 7：迁移 subscription.rs

**文件：**
- 创建：`momentum_api/src/websocket/manager/subscription.rs`
- 合并自：`momentum_api/src/websocket/subscription/manager.rs` + `topic.rs`
- 参考：`momentum_api/src/websocket/subscription/manager.rs`（完整内容）

**注意：** 此任务需要完整读取 subscription/manager.rs 和 topic.rs 的内容。

- [ ] **步骤 1：读取 subscription/manager.rs**

```bash
cat momentum_api/src/websocket/subscription/manager.rs
```

- [ ] **步骤 2：读取 subscription/topic.rs**

```bash
cat momentum_api/src/websocket/subscription/topic.rs
```

- [ ] **步骤 3：创建 subscription.rs（合并两个文件）**

将 manager.rs 和 topic.rs 的内容合并到新文件，移除重复的 `pub mod` 声明。

- [ ] **步骤 4：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -10
```

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/subscription.rs
git commit -m "refactor(websocket): migrate subscription/ to manager/subscription.rs"
```

---

## 任务 8：创建 mod.rs（Facade）

**文件：**
- 创建：`momentum_api/src/websocket/manager/mod.rs`
- 参考：`momentum_api/src/websocket/manager.rs` 剩余部分（handle_socket, should_send_to_user, 测试）

**设计**：ConnectionManager 作为共享核心，其他模块通过 `Arc<ConnectionManager>` 访问连接状态，避免 async block_on。

- [ ] **步骤 1：创建 mod.rs**

```rust
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
```

**注意**：
1. `broadcast_to_workspace` 需要传入 connection 列表，在 mod.rs 中调用 `conn.get_connections_in_workspace` 获取
2. `broadcast_issue_event_to_workspace` 同理
3. ConnectionManager 需要新增 `get_user_mut` 方法供 OfflineQueueManager 使用

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -20
```

预期：有编译错误，需要逐步修复

- [ ] **Commit**

```bash
git add momentum_api/src/websocket/manager/mod.rs
git commit -m "refactor(websocket): create WebSocketManager facade in mod.rs"
```

---

## 任务 9：删除原 manager.rs

**文件：**
- 删除：`momentum_api/src/websocket/manager.rs`

- [ ] **步骤 1：删除原文件**

```bash
rm momentum_api/src/websocket/manager.rs
```

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -30
```

预期：会有类型不匹配错误，需要在 Facade 中修复

- [ ] **Commit**

```bash
git rm momentum_api/src/websocket/manager.rs
git commit -m "refactor(websocket): remove original manager.rs"
```

---

## 任务 10：删除原 subscription/ 目录

**文件：**
- 删除：`momentum_api/src/websocket/subscription/`（整个目录）

- [ ] **步骤 1：删除目录**

```bash
rm -rf momentum_api/src/websocket/subscription/
```

- [ ] **步骤 2：验证编译**

```bash
cd momentum_api && cargo check 2>&1 | grep -E "error" | head -20
```

- [ ] **Commit**

```bash
git rm -r momentum_api/src/websocket/subscription/
git commit -m "refactor(websocket): remove subscription/ (migrated to manager/)"
```

---

## 任务 11：运行完整测试

**文件：**
- 测试：`tests/websocket/basic_tests.rs`
- 测试：`tests/websocket/stress_tests.rs`

- [ ] **步骤 1：运行 WebSocket 测试**

```bash
cd momentum_api && cargo test --test basic_tests --test stress_tests 2>&1 | tail -50
```

预期：所有测试通过

- [ ] **步骤 2：运行完整测试套件**

```bash
cargo test 2>&1 | tail -30
```

预期：所有测试通过

- [ ] **Commit（如有修复）**

```bash
git add -A && git commit -m "fix(websocket): resolve compilation/test issues"
```

---

## 验收检查

- [ ] `websocket/manager.rs` 已删除
- [ ] `websocket/subscription/` 目录已删除
- [ ] `websocket/manager/` 包含所有子模块
- [ ] 所有测试通过
- [ ] 无编译警告
