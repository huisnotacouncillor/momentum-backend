# WebSocketManager 重构设计方案

> **日期**: 2026-07-18
> **状态**: 已批准
> **目标**: 将 1124 行 `manager.rs` 拆分为职责明确的子模块

---

## 1. 背景

`momentum_api/src/websocket/manager.rs` 当前承担了过多职责：
- 连接管理（connections HashMap）
- 广播（broadcast_tx 全局 + direct_senders 精准发送）
- 断连恢复（recovery_info HashMap）
- 离线消息队列（ConnectedUser.message_queue）
- Issue 事件广播（broadcast_issue_event）
- 订阅管理（已有 `subscription/` 模块独立存在，但 manager 也有残留）

违反 SRP（单一职责原则），导致：
- 代码难以理解和维护
- 难以单独测试
- 新增功能继续膨胀此文件

---

## 2. 目标结构

```
websocket/
├── manager/                         # 新目录，WebSocketManager 拆分至此
│   ├── mod.rs                      # Unified facade
│   ├── connection.rs               # 连接生命周期
│   ├── broadcast.rs                # 广播 + 精准发送
│   ├── recovery.rs                 # 断连恢复
│   ├── offline_queue.rs            # 离线消息队列
│   └── subscription.rs             # 从 subscription/ 迁移
├── subscription/                   # ← 合并后删除
├── registry/                       # ← 保持独立
├── handlers.rs                     # ← 调用方，需更新
├── handler.rs                      # ← 调用方，需更新
└── mod.rs                          # ← 调用方，需更新
```

---

## 3. 各子模块职责

### 3.1 connection.rs — 连接生命周期

**职责**: 管理 `connections: HashMap<ConnectionId, ConnectedUser>` 的 CRUD 操作

**核心 API**:
```rust
// 添加连接
pub async fn add_connection(
    &self,
    connection_id: String,
    user: ConnectedUser,
    db: Option<&Arc<DbPool>>,
    asset_helper: Option<&Arc<AssetUrlHelper>>,
) -> Result<(), ConnectionError>

// 移除连接
pub async fn remove_connection(&self, connection_id: &str) -> Option<ConnectedUser>

// 暂停连接
pub async fn suspend_connection(&self, connection_id: &str)

// 恢复连接
pub async fn resume_connection(&self, connection_id: &str)

// 查询
pub async fn get_connection(&self, connection_id: &str) -> Option<ConnectedUser>
pub async fn get_online_users(&self) -> Vec<ConnectedUser>
pub async fn get_connection_count(&self) -> usize
pub async fn get_connections_by_user(&self, user_id: Uuid) -> Vec<String>
pub async fn get_connections_in_workspace(&self, workspace_id: Uuid) -> Vec<String>
pub async fn update_ping(&self, connection_id: &str)

// 清理过期连接
pub async fn cleanup_stale_connections(&self, timeout_minutes: i64) -> usize
```

**内部状态**:
```rust
connections: Arc<RwLock<HashMap<String, ConnectedUser>>>
```

**依赖**: 无（纯内存状态）

---

### 3.2 broadcast.rs — 广播 + 精准发送

**职责**: 消息分发，包括全局广播和精准发送

**核心 API**:
```rust
// 全局广播
pub async fn broadcast_message(&self, message: WebSocketMessage) -> Result<(), BroadcastError>

// 按工作区广播
pub async fn broadcast_to_workspace(&self, workspace_id: Uuid, message: WebSocketMessage) -> Result<(), BroadcastError>

// 精准发送（已实现）
pub async fn direct_send(&self, connection_id: &str, message: WebSocketMessage) -> Result<(), String>

// 获取广播接收器
pub fn get_broadcast_receiver(&self) -> broadcast::Receiver<WebSocketMessage>

// Issue 事件广播
pub async fn broadcast_issue_event(&self, event: IssueEvent)
pub async fn broadcast_issue_event_to_workspace(&self, workspace_id: Uuid, event: IssueEvent)
```

**内部状态**:
```rust
broadcast_tx: broadcast::Sender<WebSocketMessage>
direct_senders: Arc<RwLock<HashMap<String, broadcast::Sender<WebSocketMessage>>>>
```

**依赖**:
- `connection.rs` — 需要查询连接信息（如工作区）进行过滤
- `issue_events.rs` — 用于 Issue 事件广播

---

### 3.3 recovery.rs — 断连恢复

**职责**: 管理连接恢复信息和恢复流程

**核心 API**:
```rust
// 创建恢复信息（在 remove_connection 时调用）
pub async fn create_recovery_info(&self, user: &ConnectedUser)

// 恢复连接
pub async fn recover_connection(
    &self,
    user_id: Uuid,
    recovery_token: &str,
) -> Option<ConnectedUser>
```

**内部状态**:
```rust
recovery_info: Arc<RwLock<HashMap<Uuid, ConnectionRecoveryInfo>>>
recovery_token_ttl: Duration
```

---

### 3.4 offline_queue.rs — 离线消息队列

**职责**: 管理离线用户的消息队列

**核心 API**:
```rust
pub async fn add_offline_message(&self, user_id: Uuid, message: WebSocketMessage)
pub async fn get_offline_messages(&self, user_id: Uuid) -> VecDeque<WebSocketMessage>
```

**与 connection.rs 的关系**: offline_queue 依附于 ConnectedUser.message_queue，offline_queue.rs 提供独立的访问接口。

---

### 3.5 subscription.rs — 订阅管理

**职责**: 从 `websocket/subscription/` 迁移，负责主题订阅

**核心 API**:
```rust
pub async fn subscribe(&self, connection_id: &str, topic: Topic) -> SubscribeResult
pub async fn unsubscribe(&self, connection_id: &str, topic: Topic) -> UnsubscribeResult
pub async fn get_subscriptions(&self, connection_id: &str) -> Vec<Topic>
pub fn is_subscribed(&self, connection_id: &str, topic: &Topic) -> bool
```

**内部状态**:
```rust
topic_subs: Arc<RwLock<HashMap<Topic, HashSet<String>>>>  // topic -> connection_ids
conn_topics: Arc<RwLock<HashMap<String, HashSet<Topic>>>>  // connection_id -> topics
```

**迁移**: 将 `websocket/subscription/manager.rs` 和 `topic.rs` 合并到此文件，删除原 `subscription/` 目录。

---

## 4. mod.rs — Unified Facade

```rust
use crate::websocket::issue_events::IssueEvent;
use uuid::Uuid;

pub struct WebSocketManager {
    pub(crate) conn: connection::ConnectionManager,
    pub(crate) broadcast: broadcast::BroadcastManager,
    pub(crate) recovery: recovery::RecoveryManager,
    pub(crate) offline: offline_queue::OfflineQueueManager,
    pub(crate) subscription: subscription::SubscriptionManager,
}

impl WebSocketManager {
    pub fn new() -> Self {
        Self {
            conn: connection::ConnectionManager::new(),
            broadcast: broadcast::BroadcastManager::new(),
            recovery: recovery::RecoveryManager::new(),
            offline: offline_queue::OfflineQueueManager::new(),
            subscription: subscription::SubscriptionManager::new(),
        }
    }

    // === Connection (委托) ===
    pub async fn add_connection(...) { self.conn.add_connection(...).await }
    pub async fn remove_connection(...) { self.conn.remove_connection(...).await }
    pub async fn suspend_connection(...) { self.conn.suspend_connection(...).await }
    pub async fn resume_connection(...) { self.conn.resume_connection(...).await }
    pub async fn get_connection(...) { self.conn.get_connection(...).await }
    pub async fn get_online_users(...) { self.conn.get_online_users(...).await }
    pub async fn get_connection_count(...) { self.conn.get_connection_count(...).await }
    pub async fn update_ping(...) { self.conn.update_ping(...).await }
    pub async fn cleanup_stale_connections(...) { self.conn.cleanup_stale_connections(...).await }

    // === Broadcast (委托) ===
    pub async fn broadcast_message(...) { self.broadcast.broadcast_message(...).await }
    pub async fn broadcast_to_workspace(...) { self.broadcast.broadcast_to_workspace(...).await }
    pub async fn direct_send(...) { self.broadcast.direct_send(...).await }
    pub fn get_broadcast_receiver(...) { self.broadcast.get_broadcast_receiver(...) }
    pub async fn broadcast_issue_event(...) { self.broadcast.broadcast_issue_event(...).await }
    pub async fn broadcast_issue_event_to_workspace(...) { self.broadcast.broadcast_issue_event_to_workspace(...).await }

    // === Recovery (委托) ===
    pub async fn recover_connection(...) { self.recovery.recover_connection(...).await }

    // === Offline Queue (委托) ===
    pub async fn add_offline_message(...) { self.offline.add_offline_message(...).await }
    pub async fn get_offline_messages(...) { self.offline.get_offline_messages(...).await }

    // === Subscription (委托) ===
    pub async fn subscribe(...) { self.subscription.subscribe(...).await }
    pub async fn unsubscribe(...) { self.subscription.unsubscribe(...).await }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}
```

---

## 5. 数据流关系

```
add_connection(connection_id, user)
  1. conn.add_connection()         → 插入 connections HashMap
  2. broadcast.register_sender()   → 插入 direct_senders（由 broadcast 模块内部处理）

remove_connection(connection_id)
  1. conn.remove_connection()      → 移除 connections
  2. broadcast.unregister_sender() → 移除 direct_senders（由 broadcast 模块内部处理）
  3. recovery.create_recovery_info() → 创建恢复信息

broadcast_to_workspace(workspace_id, message)
  1. conn.get_connections_in_workspace() → 获取该工作区的 connection_ids
  2. broadcast.direct_send_batch()       → 批量精准发送

recover_connection(user_id, token)
  1. recovery.recover_connection() → 验证 token，返回 pending_messages
  2. conn.add_connection()        → 重新插入 connections
```

---

## 6. Breaking Changes

由于采用 breaking change 策略，以下调用方需要更新导入路径：

| 文件 | 旧 import | 新 import |
|------|-----------|-----------|
| `websocket/handler.rs` | `manager::{ConnectedUser, WebSocketManager}` | `manager::WebSocketManager`（ConnectedUser 保留在 manager） |
| `websocket/handlers.rs` | `manager::WebSocketManager` | `manager::WebSocketManager` |
| `websocket/mod.rs` | `manager::{ConnectedUser, MessageType, WebSocketManager, WebSocketMessage}` | `manager::{WebSocketManager, ConnectedUser, MessageType, WebSocketMessage}` |
| `websocket/registry_dispatch.rs` | 无 | 无变化 |
| `tests/websocket/basic_tests.rs` | `manager::WebSocketManager` | `manager::WebSocketManager` |
| `tests/websocket/stress_tests.rs` | `manager::WebSocketManager` | `manager::WebSocketManager` |

---

## 7. 实现顺序

1. **创建 `manager/` 目录结构**
2. **迁移 `connection.rs`** — 最独立，无依赖
3. **迁移 `broadcast.rs`** — 依赖 connection（需要查询连接信息）
4. **迁移 `recovery.rs`** — 最独立，无依赖
5. **迁移 `offline_queue.rs`** — 最独立，无依赖
6. **迁移 `subscription.rs`** — 从 `subscription/` 合并
7. **编写 `mod.rs`** — Facade 组装所有子模块
8. **更新调用方** — handler.rs, handlers.rs, mod.rs, 测试文件
9. **删除原 `manager.rs`**（确认迁移完整后）
10. **删除原 `subscription/` 目录**（确认迁移完整后）
11. **运行测试验证**

---

## 8. 测试策略

- 每个子模块独立测试
- 保留 `workspace_isolation_tests` 测试（manager.rs 末尾）
- 更新 WebSocketManager facade 测试以使用新模块
- 集成测试：端到端 WebSocket 连接流程

---

## 9. 验收标准

- [ ] `manager.rs` 拆分完毕，原文件删除
- [ ] `subscription/` 目录删除，内容合并到 `manager/subscription.rs`
- [ ] 所有现有测试通过
- [ ] 无编译警告
- [ ] 调用方导入路径更新，无 break 编译
