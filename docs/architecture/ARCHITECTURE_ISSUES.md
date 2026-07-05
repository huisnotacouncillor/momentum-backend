# Momentum Backend 架构问题分析报告

> 生成日期：2026-07-05
> 分析范围：momentum_api、momentum_core、momentum_plugin_host

---

## 目录

1. [WebSocket 命令双重分发问题](#问题1-websocket-命令双重分发)
2. [服务层可测试性差](#问题2-服务层可测试性差)
3. [WebSocket 状态管理混乱](#问题3-websocket-状态管理混乱)
4. [领域事件未连接 WebSocket](#问题4-领域事件未连接-websocket)
5. [同步数据库操作阻塞 tokio 线程](#问题5-同步数据库操作阻塞-tokio-线程)
6. [数据库连接生命周期管理](#问题6-数据库连接生命周期管理)
7. [循环依赖风险评估](#问题7-循环依赖风险评估)
8. [其他问题汇总](#其他问题汇总)

---

## 问题 1: WebSocket 命令双重分发

### 问题描述

系统存在两套并存的 WebSocket 命令分发路径：

1. **Registry 路径**（新）：基于 `HandlerRegistry` trait 的命令注册表模式
2. **Legacy 路径**（旧）：基于 `match` 语句的硬编码分发

### 代码证据

**两套 ping 处理并存：**

```rust
// Legacy: commands/handler.rs:632（实际文件，不是 websocket/handler.rs）
WebSocketCommand::Ping { .. } => Ok(serde_json::json!({"message": "pong"})),

// Registry: registry/handlers/ping.rs
pub struct PingHandler;
#[async_trait]
impl CommandHandler for PingHandler {
    fn command_type(&self) -> &'static str { "ping" }
    async fn handle(&self, ctx: RequestContext, payload: Value) -> Result<Value, HandlerError> {
        Ok(json!({ "ok": true, "echo": payload, "user_id": ctx.user_id, "ts": Utc::now() }))
    }
}
```

**两套 subscribe 处理并存：**

```rust
// Legacy: commands/handler.rs:625-628（分发调用），实际 stub 在 :912-920
WebSocketCommand::Subscribe { topics, .. } => self.handle_subscribe(ctx, topics).await,
// stub body (commands/handler.rs:912-920) 仅返回成功，不做实际订阅操作
Ok(serde_json::json!({
    "subscribed_topics": topics,
    "message": "Successfully subscribed to topics"
}))

// Registry: registry/handlers/subscribe.rs (完整实现)
pub struct SubscribeHandler { session: Arc<SubscriptionSession> }
#[async_trait]
impl CommandHandler for SubscribeHandler {
    async fn handle(&self, ctx: RequestContext, payload: Value) -> Result<Value, HandlerError> {
        // 完整的 topic 解析、验证、订阅逻辑
    }
}
```

### 关键发现：Registry 是死代码

```rust
// websocket/mod.rs:96-133
pub fn create_websocket_state(db: Arc<DbPool>, config: &Config) -> WebSocketState {
    WebSocketState {
        // ...
        command_handler: WebSocketCommandHandler::new(db.clone(), asset_helper)
            .with_message_signer(message_signer.clone()),
            // ❌ .with_registry() 从未被调用（方法存在但未调用）
            // ❌ .with_subscription_manager() 从未被调用（方法存在但未调用）
    }
}
```

**结果**：所有命令都走 Legacy 路径，Registry 基础设施完全未被激活。

### 影响

| 维度 | 影响 |
|------|------|
| **可维护性** | 两套实现需要同时维护，增加心智负担 |
| **一致性** | subscribe/unsubscribe 的 Legacy stub 与 Registry 完整实现行为不一致 |
| **技术债** | Registry 代码是 "Step 8.5" 增量重构的中间状态，长期悬而未决 |
| **复杂度** | 新开发者需要理解两套分发逻辑及 fallback 机制 |

### 命令分发对照表

| 命令类型 | Registry 路径 | Legacy 路径 | 当前状态 |
|----------|---------------|-------------|----------|
| `ping` | ✅ 有 | ✅ 有 | Legacy |
| `get_connection_info` | ✅ 有 | ✅ 有 | Legacy |
| `subscribe` | ✅ 有 (完整) | ✅ 有 (stub) | Legacy (stub) |
| `unsubscribe` | ✅ 有 (完整) | ✅ 有 (stub) | Legacy (stub) |
| 其他 ~50 个命令 | ❌ 无 | ✅ 有 | Legacy |

### 修复建议

**选项 A：启用 Registry（推荐）**
1. 在 `create_websocket_state()` 中调用 `.with_registry()` 和 `.with_subscription_manager()`
2. 迁移所有命令到 Registry 模式
3. 删除 Legacy match 块

**选项 B：回退到纯 Legacy**
1. 删除整个 `registry/` 目录及相关代码
2. 在 Legacy 路径中实现完整的 subscribe/unsubscribe
3. 简化 `CommandHandler` trait 和 `HandlerRegistry`

---

## 问题 2: 服务层可测试性差

### 问题描述

服务层采用静态方法模式且无接口抽象，导致无法在单元测试中 mock 依赖。

### 代码证据

**无接口的服务实现：**

```rust
// momentum_core/src/services/auth_service.rs
pub struct AuthService;

impl AuthService {
    pub fn login(
        conn: &mut PgConnection,
        req: &RegisterRequest,
        asset_helper: &AssetUrlHelper,
    ) -> Result<LoginResponse, AppError> {
        // 直接使用 PgConnection，无法 mock
        // 直接调用 bcrypt::hash()，无法 mock
    }
}
```

**无 trait 抽象：**

```bash
$ grep -r "trait.*Service" momentum_core/src/services/
# 无结果 - 没有任何服务接口定义
```

**IssuesService 是唯一例外（但仍不够）：**

```rust
// momentum_core/src/services/issues_service.rs
pub struct IssuesService {
    pub automation_engine: Option<Arc<AutomationEngine>>,  // 可选依赖
}

impl IssuesService {
    pub fn new() -> Self { Self { automation_engine: None } }
    pub fn with_automation_engine(automation_engine: Arc<AutomationEngine>) -> Self { ... }
}
```

但 `IssuesService` 仍然：
- 直接接收 `&mut PgConnection` 而非抽象的 repository
- 无法在不启动数据库的情况下测试业务逻辑

### 测试覆盖情况

| 服务 | 单元测试 | 测试内容 |
|------|----------|----------|
| `TeamsService` | ✅ 有 | 仅 `validate_name()` 和 `validate_team_key()` 两个纯函数 |
| `TeamMembersService` | ✅ 有 | 仅 `normalize_role()` 纯函数 |
| 其他 15 个服务 | ❌ 无 | 无任何测试 |

### 影响

| 维度 | 影响 |
|------|------|
| **测试覆盖** | 业务逻辑依赖真实数据库，无法做快速的单元测试 |
| **TDD** | 无法先写测试再实现功能，违反 TDD 原则 |
| **重构风险** | 核心业务逻辑无保护网，重构风险高 |
| **CI 效率** | 需要启动 PostgreSQL 才能运行测试，CI 时间长 |

### 修复建议

**1. 定义 Repository Traits**

```rust
// momentum_core/src/db/repositories/traits.rs
pub trait IssueRepository: Send + Sync {
    fn create(&self, conn: &mut PgConnection, new_issue: &NewIssue) -> Result<Issue, AppError>;
    fn find_by_id(&self, conn: &mut PgConnection, id: Uuid) -> Result<Option<Issue>, AppError>;
    fn list(&self, conn: &mut PgConnection, filters: &IssueFilters) -> Result<Vec<Issue>, AppError>;
    // ...
}

pub trait UserRepository: Send + Sync {
    fn find_by_email(&self, conn: &mut PgConnection, email: &str) -> Result<Option<User>, AppError>;
    // ...
}
```

**2. 服务依赖注入**

```rust
pub struct IssuesService<R: IssueRepository> {
    repo: R,
    automation_engine: Option<Arc<AutomationEngine>>,
}

impl<R: IssueRepository> IssuesService<R> {
    pub fn new(repo: R) -> Self { Self { repo, automation_engine: None } }
    pub async fn create(&self, ctx: &RequestContext, req: &CreateIssueRequest) -> Result<IssueResponse, AppError> {
        let issue = self.repo.create(conn, &new_issue)?;
        // 业务逻辑...
    }
}
```

**3. Mock 实现**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use mockall::mock;
    
    mock! {
        pub IssueRepo {}
        impl IssueRepository for IssueRepo {
            fn create(&self, conn: &mut PgConnection, new_issue: &NewIssue) -> Result<Issue, AppError>;
            fn find_by_id(&self, conn: &mut PgConnection, id: Uuid) -> Result<Option<Issue>, AppError>;
        }
    }
    
    #[test]
    fn test_create_issue_success() {
        let mut mock_repo = MockIssueRepo::new();
        mock_repo.expect_create().returning(|_, _| Ok(Issue::mock()));
        
        let service = IssuesService::new(mock_repo);
        // 测试逻辑...
    }
}
```

---

## 问题 3: WebSocket 状态管理混乱

### 问题描述

WebSocket 状态分散在多个模块，存在竞态条件、锁顺序问题和重复订阅系统。

### 状态结构分散

| 状态结构 | 文件 | 用途 |
|----------|------|------|
| `ConnectionState` | manager.rs:40 | 连接状态枚举 |
| `ConnectedUser` | manager.rs:50 | 用户会话信息 |
| `WebSocketManager` | manager.rs:74 | 连接注册表 |
| `WebSocketState` | websocket/handler.rs:19 | HTTP 处理器的全局状态 |
| `AuthenticatedUser` | auth.rs:20 | 认证后的用户信息 |
| `SubscriptionManager` | subscription/manager.rs:32 | 主题订阅管理 |

### 问题 3.1：订阅更新的非原子性

```rust
// manager.rs:315-326（subscribe 实际行号范围）
pub async fn subscribe(&self, user_id: Uuid, topic: String) {
    // 🔴 LOCK #1
    let mut connections = self.connections.write().await;
    if let Some(user) = connections.get_mut(&user_id.to_string()) {
        user.subscriptions.insert(topic.clone());
    }
    // ⚠️ LOCK RELEASED HERE
    
    // 🔴 LOCK #2 - 其他操作可能在此期间修改状态
    let mut subscriptions = self.subscriptions.write().await;
    subscriptions.entry(topic).or_insert_with(HashSet::new).insert(user_id);
}
// 注意：327-342 是 unsubscribe 函数，不是 subscribe 的一部分
```

**风险**：如果两个并发调用对同一 user_id 和 topic 进行 subscribe，状态可能不一致。

### 问题 3.2：嵌套写锁导致死锁风险

```rust
// manager.rs:219-257
pub async fn recover_connection(&self, user_id: Uuid, recovery_token: &str) -> Option<ConnectedUser> {
    // 🔴 WRITE LOCK #1
    let recovery_map = self.recovery_info.write().await;
    
    if let Some(recovery_info) = recovery_map.get(&user_id) {
        if recovery_info.recovery_token == recovery_token && recovery_info.expires_at > Utc::now() {
            // 🔴 WRITE LOCK #2 (嵌套)
            let mut connections = self.connections.write().await;
            // ...
            // 🔴 WRITE LOCK #3 (嵌套)
            let mut subscriptions = self.subscriptions.write().await;
```

**风险**：如果另一个任务尝试对同一用户执行 `add_connection` 或 `remove_connection`，会等待 `connections` 锁而形成死锁。

### 问题 3.3：双订阅系统不同步

```rust
// WebSocketManager 的订阅 (manager.rs)
subscriptions: HashMap<String, HashSet<Uuid>>  // topic -> user_ids

// SubscriptionManager 的订阅 (subscription/manager.rs)
topic_subs: HashMap<Topic, HashSet<String>>     // topic -> connection_ids
conn_topics: HashMap<String, HashSet<Topic>>    // connection_id -> topics
```

**问题**：
- `WebSocketManager::subscribe()` 按 `user_id` 索引
- `SubscriptionManager::subscribe()` 按 `connection_id` 索引
- 两者维护的是不同的订阅视图，且没有同步机制

### 问题 3.4：broadcast 误用

```rust
// manager.rs:404-422
pub async fn send_to_user(&self, user_id: Uuid, message: WebSocketMessage) {
    // ...
    if !user_connections.is_empty() {
        // 🔴 使用 broadcast_tx 发送给所有监听者，而非特定用户
        if let Err(e) = self.broadcast_tx.send(message) {
```

`send_to_user` 应该直接发送给特定用户的连接，但实际上使用了 `broadcast_tx`，导致消息发送给所有监听者。

### 问题 3.5：`should_send` 硬编码为 true

```rust
// manager.rs:822-823
let should_send = true; // 所有广播消息都发送给当前连接
```

workspace 级别的过滤逻辑（注释中提到）实际上未实现。

### 影响

| 问题 | 严重程度 | 类型 |
|------|----------|------|
| 订阅非原子更新 | 中 | 竞态条件 |
| 嵌套写锁 | **高** | 死锁风险 |
| 双订阅系统 | 中 | 设计缺陷 |
| broadcast 误用 | 中 | 功能错误 |
| should_send 硬编码 | 低 | 未实现功能 |

### 修复建议

**1. 使用单一锁或 RwLock 保护复合状态**

```rust
pub struct WebSocketManager {
    state: RwLock<WebSocketManagerState>,  // 单一锁
}

struct WebSocketManagerState {
    connections: HashMap<String, ConnectedUser>,
    subscriptions: HashMap<String, HashSet<Uuid>>,
    recovery_info: HashMap<Uuid, ConnectionRecoveryInfo>,
}
```

**2. 消除嵌套锁**

```rust
pub async fn recover_connection(&self, user_id: Uuid, recovery_token: &str) -> Option<ConnectedUser> {
    let mut state = self.state.write().await;
    // 在单个锁内完成所有操作
    // ...
}
```

**3. 统一订阅系统**

删除 `SubscriptionManager`，或在 `WebSocketManager` 内部维护完整的订阅关系，不再维护两套。

---

## 问题 4: 领域事件未连接 WebSocket

### 问题描述

系统定义了完整的事件类型（`IssueEvent`、`LabelEvent` 等）和订阅系统，但业务服务在数据变更时**不发布事件**，导致客户端无法收到实时更新。

### 代码证据

**事件类型已定义：**

```rust
// websocket/issue_events.rs（注意：events/ 路径下的版本是死代码，mod.rs:27 中被注释掉）
pub enum IssueEvent {
    Created { issue: IssueResponse, workspace_id: Uuid },
    Updated { issue: IssueResponse, changes: Vec<String>, workspace_id: Uuid },
    Deleted { issue_id: Uuid },
    StatusChanged { issue_id: Uuid, old_state: String, new_state: String },
    Assigned { issue_id: Uuid, assignee_id: Option<Uuid> },
}
```

**广播辅助函数已定义但未调用：**

```rust
// websocket/handlers.rs:49-114
pub async fn broadcast_issue_created(...) { ... }
pub async fn broadcast_issue_updated(...) { ... }
pub async fn broadcast_issue_deleted(...) { ... }
pub async fn broadcast_issue_status_changed(...) { ... }
pub async fn broadcast_issue_assigned(...) { ... }
```

**服务层不发布事件：**

```rust
// WebSocket command handler
pub async fn handle_create_issue(...) -> Result<CommandResponse> {
    let issue = IssuesService::create(&mut conn, &ctx, &payload).await?;
    // ❌ 没有调用 broadcast_issue_created()
    Ok(CommandResponse::success(issue, "Issue created"))
}
```

### 影响

| 维度 | 影响 |
|------|------|
| **实时性** | 用户创建/修改 Issue 后，其他用户看不到更新 |
| **架构** | 事件系统是残缺的，无法作为事件源架构的基础 |
| **插件系统** | 插件的 event outbox 与 WebSocket 完全隔离 |

### 修复建议

**方案 A：在 Command Handler 层发布事件（简单）**

```rust
pub async fn handle_create_issue(...) -> Result<CommandResponse> {
    let issue = IssuesService::create(&mut conn, &ctx, &payload).await?;
    
    // ✅ 发布事件
    broadcast_issue_created(&ws_state, &issue, ctx.workspace_id).await;
    
    Ok(CommandResponse::success(issue, "Issue created"))
}
```

**方案 B：服务层发布领域事件（推荐，符合 DDD）**

```rust
// 在 IssuesService 中
pub async fn create(&self, conn: &mut PgConnection, ctx: &RequestContext, req: &CreateIssueRequest) -> Result<(IssueResponse, IssueEvent)> {
    // 业务逻辑...
    let event = IssueEvent::Created { issue: issue.clone(), workspace_id: ctx.workspace_id };
    Ok((response, event))  // 返回事件元组
}
```

然后在 Command Handler 层处理事件：

```rust
let (issue, event) = IssuesService::create(...).await?;
event.emit(&ws_state).await;  // 发布到事件总线
```

---

## 问题 5: 同步数据库操作阻塞 tokio 线程

### 问题描述

使用同步的 Diesel/r2d2 连接池在 async handler 中执行数据库操作，会阻塞 tokio 工作线程。

### 代码证据

**同步服务方法：**

```rust
// momentum_core/src/services/issues_service.rs
pub fn list(
    &self,
    conn: &mut PgConnection,  // 同步连接
    ctx: &RequestContext,
    filters: &IssueFilters,
) -> Result<PaginatedIssues, AppError> {
    // diesel 同步查询 - 阻塞当前线程
    let issues = issues::table
        .filter(team_id.eq(&ctx.workspace_id))
        .limit(limit)
        .load::<Issue>(conn)?;
}
```

**同步 bcrypt 操作：**

```rust
// momentum_core/src/services/auth_service.rs
let hashed_password = hash(&req.password, bcrypt::DEFAULT_COST)  // CPU 密集型同步操作
    .map_err(|_| AppError::internal("Failed to hash password"))?;
```

**无 `spawn_blocking` 包装：**

```bash
$ grep -r "spawn_blocking" --include="*.rs" momentum_api/src/routes/
# 无结果 - 数据库操作直接在 async 上下文中执行
```

### 影响

| 维度 | 影响 |
|------|------|
| **吞吐量** | 阻塞的 DB 操作会耗尽 tokio 线程池 |
| **延迟** | 请求排队等待被阻塞的线程 |
| **bcrypt 放大** | 登录/注册的密码 hashing（CPU 密集）加重问题 |

### 修复建议

**短期：使用 `spawn_blocking`**

```rust
pub async fn login(...) -> impl IntoResponse {
    let mut conn = state.db.get().await.unwrap();  // 需要改用 async pool
    
    let result = tokio::task::spawn_blocking(move || {
        AuthService::login(&mut conn, &payload, &state.asset_helper)
    }).await??;
}
```

**长期：迁移到 async ORM**

考虑迁移到 `sqlx` + `deadpool-postgres`：
- 完全异步的连接池
- 真正的非阻塞 I/O
- 更好的 tokio 集成

---

## 问题 6: 数据库连接生命周期管理

### 问题描述

连接通过 `Drop` 隐式释放，无显式生命周期管理。

### 代码证据

**隐式释放：**

```rust
// routes/auth.rs
pub async fn login(State(state): State<Arc<AppState>>, ...) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => return internal_error(),
    };
    
    let result = AuthService::login(&mut conn, &payload, &state.asset_helper);
    // conn 在此处 Drop，自动归还池中
}
```

**连接超时配置：**

```rust
// config.rs
fn default_connection_timeout() -> u64 { 30 }  // 30 秒
fn default_max_connections() -> u32 { 20 }
```

### 影响

- 连接释放在 Drop 时发生，如果 handler 返回 `Result::Err`，连接可能在错误路径上延迟释放
- 无连接泄漏风险（r2d2 会正确管理），但无显式控制增加了不确定性

### 建议

当前实现可接受。如需更精细控制，可引入 `conn.acquire()` / `conn.release()` 显式 API。

---

## 问题 7: 循环依赖风险评估

### 问题描述

评估 `momentum_core` 依赖 Diesel 是否会形成循环依赖。

### 分析结果

**当前依赖图：**

```
momentum_api 
    ├── momentum_core (path dependency)
    │     └── diesel (FULL dependency)
    │     └── r2d2, redis, serde, etc.
    └── momentum_plugin_host (path dependency)
          └── momentum_core (path dependency)
```

**结论：无循环依赖**

`momentum_core` 不反向依赖 `momentum_api`。

### 潜在风险点

| 风险点 | 说明 | 严重程度 |
|--------|------|----------|
| `error.rs` 依赖 `db::models::api` | 错误模块依赖 API 模型层 | 低 |
| `momentum_api::validation` 依赖 `momentum_core::db::models::api::ErrorDetail` | API 层依赖 DB 模型层 | 低（但违反分层） |

### 修复建议

**1. 抽取核心 API 响应类型**

将 `ApiResponse`、`ErrorDetail` 移到 `momentum_core` 的独立模块（如 `momentum_core::api`），与 `db::models` 分离。

**2. API 层定义自己的错误类型**

```rust
// momentum_api/src/error.rs
pub struct ApiErrorDetail {
    pub field: Option<String>,
    pub code: String,
    pub message: String,
}
```

---

## 其他问题汇总

### 8.1 Plugin 系统与核心耦合

| 问题 | 说明 |
|------|------|
| 位置 | `momentum_core/src/plugins/` + `momentum_plugin_host` |
| 问题 | 插件扩展点定义与业务逻辑混合，边界不清晰 |
| 建议 | 抽取为独立 crate `momentum-plugins`，通过接口通信 |

### 8.2 重复的 Cursor 类型

`IssueCursor` 在 `momentum_core/src/db/models/issue.rs` 定义，WebSocket 层可能存在重复定义。

### 8.3 未清理的死代码

根据 git log `fix: resolve unused variable warnings in WebSocket modules`，存在未使用的变量/导入被清理，说明代码审查不够彻底。

### 8.4 测试覆盖不足

| 模块 | 测试覆盖 |
|------|----------|
| Services (16) | 仅 2 个有简单验证测试 |
| Repositories (24) | 无单元测试（依赖集成测试） |
| WebSocket handlers | 无测试 |
| Middleware | 无测试 |

---

## 总结：问题优先级矩阵

| 优先级 | 问题 | 修复成本 | 影响范围 |
|--------|------|----------|----------|
| **P0 - 立即修复** | 嵌套写锁死锁风险 | 高 | 可用性 |
| **P1 - 高优先级** | 领域事件未连接 WebSocket | 中 | 功能完整性 |
| **P1 - 高优先级** | 同步 DB 操作阻塞线程 | 高 | 性能/可扩展性 |
| **P2 - 中优先级** | WebSocket 命令双重分发 | 中 | 可维护性 |
| **P2 - 中优先级** | 服务层可测试性 | 高 | 开发效率 |
| **P2 - 中优先级** | 双订阅系统不同步 | 中 | 功能正确性 |
| **P3 - 低优先级** | 循环依赖风险 | 低 | 架构清洁度 |
| **P3 - 低优先级** | Plugin 系统耦合 | 高 | 长期可扩展性 |

---

## 附录：关键文件路径

| 文件 | 说明 |
|------|------|
| `momentum_api/src/websocket/registry_dispatch.rs` | Registry 分发实现 |
| `momentum_api/src/websocket/commands/handler.rs` | Legacy 分发实现（1794 行） |
| `momentum_api/src/websocket/handler.rs` | HTTP 处理入口（320 行，非命令分发） |
| `momentum_api/src/websocket/manager.rs` | WebSocket 连接管理 |
| `momentum_api/src/websocket/subscription/manager.rs` | 订阅管理 |
| `momentum_api/src/websocket/issue_events.rs` | Issue 事件定义（**注意**：events/ 子模块是死代码） |
| `momentum_core/src/services/` | 服务层实现 |
| `momentum_core/src/db/repositories/` | 仓库层实现 |
| `momentum_core/src/error.rs` | 错误类型定义 |
| `momentum_api/src/routes/` | HTTP 路由处理 |

---

## 勘误记录

### 2026-07-05 修订

基于代码验证（见 `ARCHITECTURE_REVIEW.md`），本文档做了以下修正：

| 原引用 | 修正后 | 原因 |
|--------|--------|------|
| `websocket/handler.rs:625-628, 632` | `commands/handler.rs:625-628, 632` | websocket/handler.rs 只有 320 行，不存在 625+ 行 |
| `events/issue_events.rs` | `issue_events.rs`（根目录） | events/ 子模块在 mod.rs:27 被注释，是死代码 |
| `manager.rs:315-342` 全属 subscribe | 拆分为 315-326 (subscribe) 和 327-342 (unsubscribe) | 行号范围混淆了两个函数 |
| Ping Handler 返回 `{status, server_time}` | 返回 `{ok, echo, user_id, ts}` | 真实实现与文档不一致 |
| Subscribe Legacy body 简化版 | 实际在 commands/handler.rs:912-920 | 文档引用的是分发点而非 stub 实现 |

### 相关文档

- `ARCHITECTURE_REVIEW.md` — 架构师审视（包含 12 维度 + 4 个重大遗漏：安全/运维/可观测性/资源效率）
- `REFACTOR_PLAN.md` — 详细修复实施计划（按 P0-P3 阶段，含代码示例）
