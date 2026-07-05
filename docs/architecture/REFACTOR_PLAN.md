# Momentum Backend 修复实施计划

> **生成日期**：2026-07-05
> **目标**：将架构审视报告转化为可执行的修复计划
> **方法**：TDD + 渐进式重构

---

## 目录

1. [P0 阶段：安全漏洞与生产阻塞](#p0-阶段安全漏洞与生产阻塞)
2. [P1 阶段：核心架构改进](#p1-阶段核心架构改进)
3. [P2 阶段：代码质量与可维护性](#p2-阶段代码质量与可维护性)
4. [P3 阶段：长期演进](#p3-阶段长期演进)
5. [执行原则](#执行原则)

---

## P0 阶段：安全漏洞与生产阻塞

> **目标**：消除 Critical 漏洞和阻止生产部署的问题
> **时间预估**：1-2 周
> **风险**：高（触碰核心业务逻辑）

### P0.1: 修复工作区隔离漏洞

**问题**：`IssueRepo::find_by_id_in_workspace`、`list_by_workspace`、`search_by_title` 忽略 `workspace_id` 参数

**修复策略**：TDD - 先写测试，再修复

```rust
// 步骤 1：编写失败的测试
#[cfg(test)]
mod tests {
    #[test]
    fn find_by_id_in_workspace_rejects_cross_workspace_access() {
        // 创建两个工作区的 issues
        let ws_a_issue = create_test_issue(workspace_a, team_a);
        let ws_b_issue = create_test_issue(workspace_b, team_b);
        
        // 用工作区 A 的 context 查询工作区 B 的 issue
        let result = IssueRepo::find_by_id_in_workspace(
            &mut conn,
            workspace_a,  // ctx.workspace_id
            ws_b_issue.id,
        ).unwrap();
        
        // 应该返回 None（不允许跨工作区访问）
        assert!(result.is_none());
    }
}
```

```rust
// 步骤 2：修复实现
pub fn find_by_id_in_workspace(
    conn: &mut PgConnection,
    workspace_id: uuid::Uuid,  // 去掉下划线
    issue_id: uuid::Uuid,
) -> Result<Option<Issue>, diesel::result::Error> {
    use crate::schema::issues::dsl::*;
    
    // 先获取该工作区的所有 team_id
    let workspace_team_ids: Vec<Uuid> = teams::table
        .filter(teams::workspace_id.eq(workspace_id))
        .select(teams::id)
        .load(conn)?;
    
    issues
        .filter(id.eq(issue_id))
        .filter(team_id.eq_any(&workspace_team_ids))  // ✅ 强制过滤
        .first::<Issue>(conn)
        .optional()
}
```

**同样修复**：
- `list_by_workspace` (issues.rs:38-44)
- `search_by_title` (issues.rs:145-156)

**审查所有类似问题**：搜索其他仓库的 `*_workspace` 函数是否有相同模式

### P0.2: switch_workspace 添加成员验证

```rust
// auth_service.rs
pub fn switch_workspace(
    conn: &mut PgConnection,
    user_id: Uuid,
    target_workspace_id: Uuid,
) -> Result<User, AppError> {
    // ✅ 新增：验证用户是该工作区的成员
    let membership = WorkspaceMembersRepo::find_by_user_and_workspace(
        conn,
        user_id,
        target_workspace_id,
    )?;
    
    if membership.is_none() {
        return Err(AppError::Forbidden {
            message: "User is not a member of this workspace".to_string(),
        });
    }
    
    AuthRepo::update_current_workspace(conn, user_id, target_workspace_id)
}
```

### P0.3: WebSocket 广播添加工作区过滤

```rust
// manager.rs - handle_socket 中的 send_task
let should_send = match &message.message_type {
    MessageType::WorkspaceEvent { workspace_id } => {
        user.current_workspace_id.as_ref() == Some(workspace_id)
    }
    MessageType::UserEvent { user_id } => {
        &user.user_id == user_id
    }
    MessageType::Broadcast => true,
    _ => false,  // 未知类型默认不发送
};
```

### P0.4: 实现 RBAC 中间件

**步骤 1：定义权限 trait**

```rust
// middleware/permission.rs
pub trait Permission: Send + Sync {
    fn workspace_id(&self) -> Uuid;
    fn user_id(&self) -> Uuid;
}

pub async fn require_workspace_role<R: WorkspaceRole>(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
    user: AuthUserInfo,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let mut conn = state.db.get().await?;
    
    let membership = WorkspaceMembersRepo::find_by_user_and_workspace(
        &mut conn,
        user.user_id,
        workspace_id,
    )?;
    
    let role = membership
        .ok_or(AppError::Forbidden { 
            message: "Not a workspace member".to_string() 
        })?
        .role;
    
    R::check(role)?;
    
    Ok(next.run(request).await)
}

pub trait WorkspaceRole {
    fn check(role: WorkspaceMemberRole) -> Result<(), AppError>;
}

pub struct RequireAdmin;
impl WorkspaceRole for RequireAdmin {
    fn check(role: WorkspaceMemberRole) -> Result<(), AppError> {
        match role {
            WorkspaceMemberRole::Owner | WorkspaceMemberRole::Admin => Ok(()),
            _ => Err(AppError::Forbidden { 
                message: "Admin role required".to_string() 
            }),
        }
    }
}

pub struct RequireOwner;
impl WorkspaceRole for RequireOwner {
    fn check(role: WorkspaceMemberRole) -> Result<(), AppError> {
        match role {
            WorkspaceMemberRole::Owner => Ok(()),
            _ => Err(AppError::Forbidden { 
                message: "Owner role required".to_string() 
            }),
        }
    }
}
```

**步骤 2：应用到受保护的路由**

```rust
// routes/workspaces.rs
.route(
    "/workspaces/:workspace_id",
    delete(require_workspace_role::<RequireOwner> 
        |> delete_workspace_handler)
)
.route(
    "/workspaces/:workspace_id/members",
    post(require_workspace_role::<RequireAdmin>
        |> add_member_handler)
)
```

### P0.5: 修复连接池 panic

```rust
// middleware/auth.rs (修复前)
let mut conn = pool.get().expect("Failed to get DB connection");

// middleware/auth.rs (修复后)
let mut conn = pool.get().map_err(|_| {
    AppError::ServiceUnavailable { 
        message: "Database temporarily unavailable".to_string() 
    }
})?;
```

**同时修复**：`websocket/auth.rs:167, 190`

### P0.6: 添加 /health 端点

```rust
// routes/health.rs
use axum::{http::StatusCode, response::IntoResponse, extract::State};
use std::sync::Arc;
use crate::AppState;

pub async fn health_check(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut checks = Vec::new();
    
    // 检查 DB
    match state.db.get() {
        Ok(_) => checks.push(("database", "ok")),
        Err(e) => checks.push(("database", &format!("error: {}", e))),
    }
    
    // 检查 Redis
    match state.redis.get_multiplexed_async_connection().await {
        Ok(_) => checks.push(("redis", "ok")),
        Err(e) => checks.push(("redis", &format!("error: {}", e))),
    }
    
    let all_ok = checks.iter().all(|(_, status)| *status == "ok");
    let status = if all_ok { 
        StatusCode::OK 
    } else { 
        StatusCode::SERVICE_UNAVAILABLE 
    };
    
    (status, serde_json::json!({
        "status": if all_ok { "healthy" } else { "unhealthy" },
        "checks": checks,
    }))
}
```

```rust
// routes/mod.rs - 注册
.route("/health", get(health_check))
.route("/ready", get(readiness_check))
```

### P0.7: 修复 Dockerfile

```dockerfile
# 安装 curl 用于 healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    && rm -rf /var/lib/apt/lists/*

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1
```

### P0 阶段验证清单

- [ ] 所有 IssueRepo 的 `*_workspace` 函数通过跨工作区拒绝测试
- [ ] switch_workspace 无权限时返回 403
- [ ] WebSocket broadcast_to_workspace 验证过滤逻辑
- [ ] RBAC middleware 部署到关键路由
- [ ] 连接池耗尽返回 503 而非 panic
- [ ] `/health` 端点返回 200
- [ ] Dockerfile healthcheck 成功

---

## P1 阶段：核心架构改进

> **目标**：修复架构核心问题，建立可靠性基础
> **时间预估**：3-4 周
> **风险**：中

### P1.1: 实现 graceful shutdown

```rust
// main.rs
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ... 现有初始化
    
    let app = Router::new()...;
    
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    
    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    
    tracing::info!("Server shut down gracefully");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };
    
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };
    
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    
    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C"),
        _ = terminate => tracing::info!("Received SIGTERM"),
    }
    
    tracing::info!("Starting graceful shutdown...");
}
```

### P1.2: 修复严重 N+1 查询

**目标**：`build_issue_response` 减少 80%+ 查询次数

```rust
// issues_service.rs - 新增批量加载
pub struct IssueBatchContext {
    pub teams: HashMap<Uuid, Team>,
    pub projects: HashMap<Uuid, Project>,
    pub project_statuses: HashMap<Uuid, ProjectStatus>,
    pub users: HashMap<Uuid, User>,
    pub workspace_statuses: Vec<ProjectStatus>,
    pub workflow_states: HashMap<Uuid, Vec<WorkflowState>>,
    pub labels: HashMap<Uuid, Vec<Label>>,
    pub cycles: HashMap<Uuid, Cycle>,
}

impl IssueBatchContext {
    pub fn load_for_issues(
        conn: &mut PgConnection,
        workspace_id: Uuid,
        issues: &[Issue],
    ) -> Result<Self, AppError> {
        // 一次性加载所有相关数据
        let team_ids: Vec<Uuid> = issues.iter().map(|i| i.team_id).collect();
        let teams = TeamsRepo::list_by_ids(conn, &team_ids)?;
        
        // ... 类似批量加载其他关联数据
        
        Ok(Self { ... })
    }
}

pub fn build_issue_response(
    issue: &Issue,
    ctx: &IssueBatchContext,
) -> IssueResponse {
    // 从 ctx 中查找，不再单独查询
    IssueResponse::from_context(issue, ctx)
}
```

### P1.3: 修复同步 DB 操作阻塞问题

**短期方案**：使用 `spawn_blocking`

```rust
// 通用包装器
pub async fn run_db<F, R>(pool: Arc<DbPool>, f: F) -> Result<R, AppError>
where
    F: FnOnce(&mut PgConnection) -> Result<R, AppError> + Send + 'static,
    R: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(AppError::Pool)?;
        f(&mut conn)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?
}
```

**长期方案**：迁移到 sqlx（独立项目）

### P1.4: 修复嵌套锁死锁风险

```rust
// manager.rs - 重构 recover_connection
pub async fn recover_connection(
    &self, 
    user_id: Uuid, 
    recovery_token: &str
) -> Option<ConnectedUser> {
    // ✅ 单一写锁内完成所有操作
    let mut state = self.state.write().await;
    
    // 验证 token
    let recovery_info = state.recovery_info.get(&user_id)?;
    if recovery_info.recovery_token != recovery_token 
        || recovery_info.expires_at <= Utc::now() {
        return None;
    }
    
    // 获取连接
    let mut user = state.connections.remove(&recovery_info.connection_id)?;
    
    // 更新订阅（在同一锁内）
    for topic in &user.subscriptions {
        if let Some(subs) = state.subscriptions.get_mut(topic) {
            subs.insert(user_id);
        }
    }
    
    user.state = ConnectionState::Connected;
    state.connections.insert(recovery_info.connection_id.clone(), user.clone());
    
    Some(user)
}
```

### P1.5: 修复搜索查询使用 GIN 索引

```rust
// repositories/issues.rs
pub fn search_by_title(
    conn: &mut PgConnection,
    workspace_id: Uuid,
    query: &str,
) -> Result<Vec<Issue>, diesel::result::Error> {
    use crate::schema::issues::dsl::*;
    use diesel::sql_types::Text;
    
    let workspace_team_ids: Vec<Uuid> = teams::table
        .filter(teams::workspace_id.eq(workspace_id))
        .select(teams::id)
        .load(conn)?;
    
    issues
        .filter(team_id.eq_any(&workspace_team_ids))
        .filter(
            // ✅ 使用预计算的 search_vector 列
            search_vector.matches(
                websearch_to_tsquery(query)
            )
        )
        .load::<Issue>(conn)
}
```

### P1.6: 启用 EnvFilter 日志配置

```rust
// main.rs
fn init_logging(config: &Config) {
    use tracing_subscriber::{EnvFilter, fmt};
    
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_target(true))
        .init();
}
```

### P1.7: 修复 trace_id 传播

```rust
// services/context.rs
pub struct RequestContext {
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    pub trace_id: String,  // ✅ 新增
    pub idempotency_key: Option<String>,
}

// middleware/request_tracking.rs
let request_id = uuid::Uuid::new_v4().to_string();
tracing::Span::current().record("trace_id", &request_id.as_str());

// 注入到 extensions
request.extensions_mut().insert(RequestContext {
    trace_id: request_id,
    // ...
});
```

### P1 阶段验证清单

- [ ] SIGTERM 触发优雅关闭（in-flight 请求完成）
- [ ] 20 个 Issue 列表只触发 < 5 次 DB 查询
- [ ] `tokio::task::spawn_blocking` 包装所有 DB 操作
- [ ] `recover_connection` 在单锁内完成
- [ ] 搜索查询使用 `search_vector` 索引（EXPLAIN 验证）
- [ ] 日志过滤按 `RUST_LOG` 工作
- [ ] DB 错误日志携带 trace_id

---

## P2 阶段：代码质量与可维护性

> **目标**：消除技术债，提升可测试性
> **时间预估**：4-6 周
> **风险**：中（大规模重构）

### P2.1: 修复 WebSocket 命令双重分发

**决策**：保留 Registry，迁移所有命令

```rust
// 步骤 1：在 WebSocketCommandHandler 中启用 Registry
impl WebSocketCommandHandler {
    pub fn new(db: Arc<DbPool>, asset_helper: AssetUrlHelper) -> Self {
        let mut registry = HandlerRegistry::new();
        let subscription_manager = SubscriptionManager::new();
        
        // 注册所有命令
        Self::register_all_handlers(&mut registry, &subscription_manager);
        
        Self {
            db,
            registry: Some(registry),
            subscription_manager: Some(subscription_manager),
            // ...
        }
    }
    
    fn register_all_handlers(
        registry: &mut HandlerRegistry,
        sub_mgr: &Arc<SubscriptionManager>,
    ) {
        registry.register(PingHandler);
        registry.register(GetConnectionInfoHandler);
        registry.register(SubscribeHandler::new(sub_mgr.clone()));
        registry.register(UnsubscribeHandler::new(sub_mgr.clone()));
        
        // Issue handlers
        registry.register(CreateIssueHandler::new(...));
        registry.register(UpdateIssueHandler::new(...));
        // ... 所有命令
    }
}

// 步骤 2：删除 Legacy match 块
```

### P2.2: 引入 Repository Traits

```rust
// db/repositories/traits.rs
#[async_trait]
pub trait IssueRepository: Send + Sync {
    async fn find_by_id(
        &self, 
        workspace_id: Uuid,
        issue_id: Uuid
    ) -> Result<Option<Issue>, AppError>;
    
    async fn list(
        &self,
        workspace_id: Uuid,
        filters: &IssueFilters,
    ) -> Result<PaginatedIssues, AppError>;
    
    async fn create(
        &self,
        workspace_id: Uuid,
        request: &CreateIssueRequest,
    ) -> Result<Issue, AppError>;
}

#[async_trait]
impl IssueRepository for IssueRepo {
    async fn find_by_id(
        &self,
        workspace_id: Uuid,
        issue_id: Uuid,
    ) -> Result<Option<Issue>, AppError> {
        let pool = self.pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get().map_err(AppError::Pool)?;
            IssueRepo::find_by_id_in_workspace(&mut conn, workspace_id, issue_id)
                .map_err(AppError::Database)
        }).await.map_err(|e| AppError::Internal(e.to_string()))?
    }
}
```

### P2.3: 服务层依赖注入

```rust
pub struct IssuesService<R: IssueRepository, A: AutomationEngineTrait> {
    repo: R,
    automation: Option<Arc<A>>,
}

impl<R: IssueRepository, A: AutomationEngineTrait> IssuesService<R, A> {
    pub fn new(repo: R) -> Self {
        Self { repo, automation: None }
    }
    
    pub fn with_automation(mut self, automation: Arc<A>) -> Self {
        self.automation = Some(automation);
        self
    }
    
    pub async fn create(
        &self,
        ctx: &RequestContext,
        req: &CreateIssueRequest,
    ) -> Result<IssueResponse, AppError> {
        let issue = self.repo.create(ctx.workspace_id, req).await?;
        
        if let Some(automation) = &self.automation {
            automation.handle_issue_created(&issue).await?;
        }
        
        Ok(issue.into())
    }
}
```

### P2.4: 统一订阅系统

**决策**：删除 `SubscriptionManager`，在 `WebSocketManager` 中维护完整订阅关系

```rust
pub struct WebSocketManager {
    state: RwLock<ManagerState>,  // 单一锁
}

struct ManagerState {
    connections: HashMap<String, ConnectedUser>,
    recovery_info: HashMap<Uuid, ConnectionRecoveryInfo>,
    topic_subscriptions: HashMap<String, HashSet<String>>,  // topic -> connection_ids
    connection_subscriptions: HashMap<String, HashSet<String>>,  // connection_id -> topics
}

impl WebSocketManager {
    pub async fn subscribe(
        &self,
        connection_id: &str,
        topics: Vec<String>,
    ) -> Result<(), AppError> {
        let mut state = self.state.write().await;
        let user = state.connections.get_mut(connection_id)
            .ok_or(AppError::NotFound { resource: "connection".into() })?;
        
        for topic in topics {
            state.topic_subscriptions
                .entry(topic.clone())
                .or_default()
                .insert(connection_id.to_string());
            user.subscriptions.insert(topic);
        }
        
        Ok(())
    }
}
```

### P2.5: 实现真正的幂等性

```rust
pub struct IdempotencyStore {
    redis: RedisClient,
    ttl: Duration,
}

impl IdempotencyStore {
    pub async fn check_and_set(
        &self,
        key: &str,
        fingerprint: &str,
    ) -> Result<IdempotencyResult, AppError> {
        // 使用 SETNX 保证原子性
        let stored: Option<String> = redis::cmd("SET")
            .arg(format!("idem:{}", key))
            .arg(fingerprint)
            .arg("NX")
            .arg("EX")
            .arg(self.ttl.as_secs())
            .query_async(&mut self.redis.get_connection().await?)
            .await?;
        
        if stored.is_none() {
            // 已存在，返回之前的响应
            let cached: Option<String> = redis::cmd("GET")
                .arg(format!("idem:resp:{}", key))
                .query_async(&mut self.redis.get_connection().await?)
                .await?;
            return Ok(IdempotencyResult::Replay(cached));
        }
        
        Ok(IdempotencyResult::Fresh)
    }
    
    pub async fn store_response(
        &self,
        key: &str,
        response: &str,
    ) -> Result<(), AppError> {
        redis::cmd("SET")
            .arg(format!("idem:resp:{}", key))
            .arg(response)
            .arg("EX")
            .arg(self.ttl.as_secs())
            .query_async(&mut self.redis.get_connection().await?)
            .await?;
        Ok(())
    }
}
```

### P2.6: 修复 Redis KEYS * 使用 SCAN

```rust
pub async fn list_user_cache_keys(
    redis: &RedisClient,
    prefix: &str,
) -> Result<Vec<String>, AppError> {
    let mut conn = redis.get_connection().await?;
    let mut keys = Vec::new();
    let mut cursor: u64 = 0;
    
    loop {
        let (next_cursor, batch): (u64, Vec<String>) = redis::cmd("SCAN")
            .arg(cursor)
            .arg("MATCH")
            .arg(format!("{}*", prefix))
            .arg("COUNT")
            .arg(100)
            .query_async(&mut conn)
            .await?;
        
        keys.extend(batch);
        cursor = next_cursor;
        
        if cursor == 0 { break; }
    }
    
    Ok(keys)
}
```

### P2.7: 修复 JWT 中间件密钥回退

```rust
// middleware/auth.rs
pub fn create_auth_layer(config: &AppConfig) -> AuthLayer {
    // ✅ 严格从配置读取，禁止回退
    let secret = config.jwt_secret.clone();
    
    if secret == "your-secret-key" || secret.is_empty() {
        panic!("JWT_SECRET must be set to a secure value");
    }
    
    AuthLayer::new(secret)
}
```

### P2.8: 实现熔断器

```rust
use failsafe::{backoff, failure_policy, Config, Instrument};

pub struct PluginClient {
    circuit_breaker: CircuitBreaker,
}

impl PluginClient {
    pub async fn call_plugin(&self, request: PluginRequest) -> Result<PluginResponse, AppError> {
        self.circuit_breaker.call(async {
            // gRPC call
            self.inner.call(request).await
        }).await.map_err(|e| match e {
            Error::Rejected => AppError::ServiceUnavailable { 
                message: "Plugin circuit breaker open".into() 
            },
            _ => AppError::Internal(format!("Plugin call failed: {}", e)),
        })
    }
}
```

### P2 阶段验证清单

- [ ] 所有命令通过 Registry 分发
- [ ] Repository trait 实现 + mock 测试
- [ ] 服务层单元测试覆盖率 > 70%
- [ ] 单一订阅系统，删除 SubscriptionManager
- [ ] Idempotency 在 Redis 中真实工作
- [ ] SCAN 替换 KEYS *
- [ ] JWT 密钥回退路径全部移除
- [ ] 熔断器在故障时打开

---

## P3 阶段：长期演进

> **目标**：建立可持续演进的基础
> **时间预估**：1-3 个月
> **风险**：低（独立项目）

### P3.1: 引入 API 版本化

```rust
// routes/mod.rs
pub fn create_router() -> Router {
    Router::new()
        .nest("/v1", v1_routes())
        .nest("/v2", v2_routes())
        // 默认重定向到 v1
        .fallback(v1_redirect)
}

fn v1_routes() -> Router {
    Router::new()
        .route("/auth/login", post(auth::login_v1))
        .route("/issues", get(issues::list_v1))
        // ...
}
```

### P3.2: 实现 Prometheus 指标

```rust
use prometheus::{Registry, Counter, Histogram, register_counter_with_registry};

pub struct Metrics {
    pub http_requests_total: CounterVec,
    pub http_request_duration: HistogramVec,
    pub ws_connections_active: Gauge,
    pub db_query_duration: HistogramVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();
        Self {
            http_requests_total: register_counter_vec!(
                "http_requests_total",
                "Total HTTP requests",
                &["method", "path", "status"]
            ).unwrap(),
            // ...
        }
    }
}

// main.rs
.route("/metrics", get(prometheus_handler))
```

### P3.3: 添加 OpenTelemetry 追踪

```rust
use opentelemetry::trace::TracerProvider;

pub fn init_tracing() -> Tracer {
    let provider = opentelemetry_otlp::new_pipeline()
        .with_endpoint("http://otel-collector:4317")
        .install_batch(opentelemetry::runtime::Tokio)?;
    
    let tracer = provider.tracer("momentum-backend");
    
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer);
    
    tracing_subscriber::registry()
        .with(telemetry)
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();
    
    tracer
}
```

### P3.4: 迁移到 sqlx（独立项目）

**阶段 1**：新功能使用 sqlx
**阶段 2**：迁移关键路径（IssuesService）
**阶段 3**：全量迁移

### P3.5: 插件系统独立化

```toml
# workspace Cargo.toml
[workspace]
members = [
    "momentum_core",
    "momentum_api",
    "momentum_plugin_host",
    "momentum_plugins_core",  # ✅ 新独立 crate
]
```

### P3.6: 添加 ADR（架构决策记录）

```markdown
# docs/adr/0001-use-axum.md

## Status
Accepted

## Context
We need a Rust web framework...

## Decision
We use Axum 0.6 because...

## Consequences
- Pro: ...
- Con: ...
```

### P3 阶段验证清单

- [ ] API 版本路由工作（`/v1`, `/v2`）
- [ ] `/metrics` 暴露 Prometheus 指标
- [ ] OpenTelemetry trace 传播到 DB
- [ ] 新服务使用 sqlx
- [ ] momentum_plugins_core 独立 crate
- [ ] 至少 3 个 ADR 文档

---

## 执行原则

### 测试先行 (TDD)

每个修复必须按以下顺序：

1. **Red**: 写一个失败的测试，演示 bug
2. **Green**: 写最小代码使测试通过
3. **Refactor**: 在测试保护下重构

### 小步提交

每个修复分解为多个可独立提交的 PR：

```
PR 1: 添加失败的测试（不修复）
PR 2: 修复实现
PR 3: 添加边界测试
PR 4: 重构（如果需要）
```

### 审查清单

每个 PR 必须包含：

- [ ] 单元测试（覆盖新代码）
- [ ] 集成测试（如果涉及跨模块）
- [ ] 更新文档（API、ADR）
- [ ] 通过 `cargo clippy`
- [ ] 通过 `cargo fmt`
- [ ] 通过所有现有测试

### 回滚策略

- 每个 PR 单独可回滚
- 关键修复使用 feature flag
- 数据库变更提供 `down.sql`

### 监控

每个阶段完成后：

1. 运行性能基准测试
2. 检查关键指标（错误率、延迟、吞吐量）
3. 灰度部署（如可能）
4. 监控 24-48 小时

---

## 时间线总览

```
Week 1-2:    P0.1 - P0.7 (安全 + 生产阻塞)
Week 3-6:    P1.1 - P1.7 (核心架构)
Week 7-12:   P2.1 - P2.8 (代码质量)
Week 13-24:  P3.1 - P3.6 (长期演进)
```

总计：约 6 个月达到生产就绪状态

---

## 文档清单

执行本计划需同步维护的文档：

- `docs/architecture/ARCHITECTURE_ISSUES.md` — 问题清单（已完成）
- `docs/architecture/ARCHITECTURE_REVIEW.md` — 架构审视（已完成）
- `docs/architecture/REFACTOR_PLAN.md` — 本文档
- `docs/adr/` — 架构决策记录（待创建）
- `docs/architecture/diagrams/` — 架构图（待创建）
- `docs/runbooks/` — 运维手册（待创建）

---

**计划生成日期**：2026-07-05
**预计完成日期**：2026-12-31
**负责人**：架构组
**审查周期**：每 2 周审查一次进度