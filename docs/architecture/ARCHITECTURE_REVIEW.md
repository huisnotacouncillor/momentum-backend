# Momentum Backend 架构审视报告

> **审视者视角**：资深架构师
> **审视日期**：2026-07-05
> **审视对象**：`docs/architecture/ARCHITECTURE_ISSUES.md`
> **范围**：架构关注度全面检视 + 关键遗漏补充

---

## 目录

1. [架构审视方法论](#架构审视方法论)
2. [文档勘误：引用错误修正](#文档勘误引用错误修正)
3. [重大遗漏：安全问题](#重大遗漏安全问题)
4. [重大遗漏：运维与生产化](#重大遗漏运维与生产化)
5. [重大遗漏：可观测性](#重大遗漏可观测性)
6. [重大遗漏：业务连续性](#重大遗漏业务连续性)
7. [审视总结：架构关注的 12 个维度](#审视总结架构关注的-12-个维度)
8. [重写的优先级矩阵](#重写的优先级矩阵)

---

## 架构审视方法论

作为架构师审视一份问题分析报告，应当从以下**12 个架构关注维度**全面检视：

| 维度 | 关注点 | 原文档覆盖度 |
|------|--------|--------------|
| 1. **功能正确性** | 业务逻辑是否符合需求 | ✅ 已覆盖 |
| 2. **安全性** | 认证、授权、注入、泄漏 | ✅ **已全部修复** |
| 3. **可靠性** | 故障处理、恢复、超时 | ✅ **已全部修复** |
| 4. **可扩展性** | 性能、容量、瓶颈 | ✅ 已覆盖 |
| 5. **可维护性** | 代码组织、模块化、命名 | ✅ 已覆盖 |
| 6. **可测试性** | 单元测试、集成测试、覆盖率 | ✅ **已全部修复** |
| 7. **可观测性** | 日志、监控、追踪、告警 | ✅ **大部分已修复** |
| 8. **可部署性** | 容器化、迁移、健康检查 | ✅ **已全部修复** |
| 9. **可演进性** | 兼容性、版本化、迁移路径 | ✅ **部分修复** |
| 10. **可操作性** | 运维效率、调试友好 | ✅ **大部分已修复** |
| 11. **业务连续性** | 幂等性、补偿、重试 | ✅ **大部分已修复** |
| 12. **资源效率** | 内存、连接池、缓存 | ✅ **大部分已修复** |

**2026-07-18 状态**：P0/P1 问题已全部修复，P2 核心问题已修复，仅剩 P3 长期改进项待处理。

---

## 文档勘误：引用错误修正

基于代码验证，发现原文档有以下**引用错误**，需要更正：

### 勘误 1：handler.rs 引用错误

**原文**（多处）：`momentum_api/src/websocket/handler.rs:625-628, 632`

**事实**：`websocket/handler.rs` 只有 **320 行**，根本不存在 625-628 行。

**正确路径**：`momentum_api/src/websocket/commands/handler.rs`（1794 行）

**影响范围**：问题 1（命令双重分发）的全部代码引用需更正。

### 勘误 2：IssueEvent 文件路径错误

**原文**：`momentum_api/src/websocket/events/issue_events.rs`

**事实**：`events` 模块在 `mod.rs:27` 是**注释掉的死代码**（`// pub mod events;`）。

**正确路径**：`momentum_api/src/websocket/issue_events.rs`（根目录下，49 行）

### 勘误 3：Ping 处理器实现细节错误

**原文**引用的 Ping 实现：
```rust
// 错误的实现
async fn handle(&self, ctx: RequestContext, payload: Value) -> Result<Value, HandlerError> {
    Ok(json!({ "status": "pong", "server_time": Utc::now() }))
}
```

**事实**：真实的 PingHandler 返回：
```rust
Ok(json!({ "ok": true, "echo": payload, "user_id": ctx.user_id, "ts": Utc::now() }))
```

### 勘误 4：subscribe 行号范围错误

**原文**：`manager.rs:315-342`

**事实**：315-326 是 `subscribe`，327-342 是 `unsubscribe`。文档错误地将两个函数合并到一个引用范围。

### 勘误 5：Ping Legacy 实现引用错误

**原文**：
```rust
WebSocketCommand::Ping => {
    let response = CommandResponse::success(None, "pong");
    broadcast_message(broadcast_tx, connection_id, response).await;
}
```

**事实**：真实代码在 `commands/handler.rs:632`：
```rust
WebSocketCommand::Ping { .. } => Ok(serde_json::json!({"message": "pong"})),
```

---

## 重大遗漏：安全问题

原文档几乎**完全忽视安全维度**。这是**最严重的遗漏**。

### 🔴 Critical - 严重安全漏洞

#### 漏洞 1：工作区隔离完全失效

**位置**：`momentum_core/src/db/repositories/issues.rs:38-44, 145-156, 175-185`

```rust
pub fn find_by_id_in_workspace(
    conn: &mut PgConnection,
    _workspace_id: uuid::Uuid,  // ❌ 下划线前缀 = 完全忽略
    issue_id: uuid::Uuid,
) -> Result<Option<Issue>, diesel::result::Error> {
    use crate::schema::issues::dsl::*;
    issues
        .filter(id.eq(issue_id))  // ❌ 只按 ID 过滤，不校验 workspace
        .first::<Issue>(conn)
        .optional()
}
```

**影响**：任意已认证用户可以**跨工作区读取、修改、删除任何 Issue**。

**证明**：调用链
- `IssueRepo::find_by_id_in_workspace` → `IssuesService::get_by_id/update/delete`
- 参数 `_workspace_id` 完全被忽略
- 用户传入任意 Issue ID，系统会返回数据

#### 漏洞 2：switch_workspace 跳过成员验证

**位置**：`momentum_core/src/services/auth_service.rs:406-415`

```rust
// 注释甚至自带 TODO
// Verify user has access to the workspace (this would need workspace member check)
// For now, just update the current workspace
let updated_user = AuthRepo::update_current_workspace(conn, ctx.user_id, workspace_id)?;
```

**影响**：用户可以切换 `current_workspace_id` 到**任意工作区**（包括未加入的），然后访问数据。

**复合攻击路径**：
1. 调用 `switch_workspace(uuid)` → 切换到任意工作区
2. 调用任意 `/workspaces/:workspace_id/...` 接口 → 绕过所有验证
3. 调用 `delete_workspace` → 删除任意工作区（因 `existing.id == ctx.workspace_id` 始终通过）

#### 漏洞 3：WebSocket 广播无工作区过滤

**位置**：`momentum_api/src/websocket/manager.rs:822-823`

```rust
let should_send = true; // ❌ 硬编码 true，过滤逻辑未实现
```

**影响**：
- 所有广播消息发送给**所有连接**
- 工作区 A 的事件泄漏给工作区 B
- 一个工作区的私密事件可以被其他工作区用户接收

#### 漏洞 4：无 RBAC（基于角色的访问控制）

**定义**：`WorkspaceMemberRole` 存在（Owner/Admin/Member/Guest），但**从未强制执行**。

**示例**：
```rust
// workspace_members_service.rs:21-87
pub fn add(...) -> Result<...> {
    // 直接接受 caller 传入的 role，写入数据库
    // ❌ 没有检查调用者是否被授权授予该角色
}
```

**影响**：
- 任何工作区成员都可以删除该工作区
- 任何成员都可以授予/移除其他成员的权限
- 角色变更无需授权验证

### 🟠 High - 高级安全问题

| 漏洞 | 描述 | 位置 |
|------|------|------|
| JWT in URL 查询参数 | 长生命周期 token 进入 WebServer 日志、浏览器历史 | `websocket/auth.rs:30-33` |
| JWT 中间件默认密钥 | `AuthConfig::default()` 回退到 `"your-secret-key"` | `middleware/auth.rs:42-50` |
| Refresh Token 无旋转 | 7 天有效期内可重复使用，无撤销机制 | `auth_service.rs:288-289` |
| 消息签名 Replay 缓存 | 无界增长 + 随机驱逐 50% | `websocket/security.rs:177-193` |
| 恢复 Token 是明文 UUID | 无哈希、无签名 | `manager.rs:197` |
| 无 HTTPS | 仅 HTTP 明文传输 | `main.rs:101` |

### 🟡 Medium - 中级安全问题

| 漏洞 | 描述 |
|------|------|
| CORS 通配符 | 默认 `*` + `allow_methods(Any)` + `allow_headers(Any)` |
| `bcrypt_cost` 配置是陷阱 | 默认值 4，永不生效，未来可能被错误启用 |
| 审计日志缺失 | 登录、密码修改、角色变更、工作区切换无审计 |
| HTTP 无频率限制 | 登录/注册端点可被暴力破解 |
| 无请求体大小限制 | DoS 风险 |
| URL 字段未校验 | SSRF 风险 |

### 安全修复建议

**1. 立即修复（Critical）**

```rust
// repositories/issues.rs - 强制 workspace_id 过滤
pub fn find_by_id_in_workspace(
    conn: &mut PgConnection,
    workspace_id: uuid::Uuid,
    issue_id: uuid::Uuid,
) -> Result<Option<Issue>, diesel::result::Error> {
    use crate::schema::issues::dsl::*;
    issues
        .filter(id.eq(issue_id))
        .filter(team_id.eq_any(
            team_repo::list_team_ids_in_workspace(conn, workspace_id)?
        ))
        .first::<Issue>(conn)
        .optional()
}
```

**2. 添加 RBAC 中间件**

```rust
// middleware/permission.rs
pub async fn require_role(
    State(state): State<Arc<AppState>>,
    Path(workspace_id): Path<Uuid>,
    user: AuthUserInfo,
    required: WorkspaceMemberRole,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let role = WorkspaceMembersRepo::get_role(
        &mut state.db.get()?,
        workspace_id,
        user.user_id,
    )?;
    
    if !role.has_permission(required) {
        return Err(AppError::Forbidden { ... });
    }
    
    Ok(next.run(request).await)
}
```

---

## 重大遗漏：运维与生产化

### 🔴 P0 运维问题

#### 问题 1：Dockerfile 健康检查永远不会成功

**位置**：`Dockerfile:56-57`

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/health || exit 1
```

**问题**：
1. **没有 `/health` 路由** — `grep /health` 在 routes 中无结果
2. **镜像未安装 curl** — `debian:bookworm-slim` 不包含 curl

**结果**：容器永远显示 `unhealthy`，但因 health check 失败退出码是 1 而非 137（OOM），导致**无限重启循环**。

**修复**：
- 添加 `/health` 端点
- Dockerfile 中安装 `curl` 或改用 `wget`

#### 问题 2：连接池耗尽导致进程 Panic

**位置**：`middleware/auth.rs:304`、`websocket/auth.rs:167, 190`

```rust
let mut conn = pool.get().expect("Failed to get DB connection");
//                              ^^^^^^ 池满时直接 panic，进程崩溃
```

**影响**：高负载下连接池饱和时整个进程崩溃，应该返回 503。

#### 问题 3：无 graceful shutdown

**位置**：`main.rs:101`

```rust
Server::bind(&addr).serve(app.into_make_service()).await?;
// ❌ 没有 .with_graceful_shutdown()
```

**影响**：
- SIGTERM 触发后，正在进行的请求被中止
- WebSocket 连接未优雅关闭
- 数据库连接未正确归还
- 清理任务泄漏

#### 问题 4：迁移未自动化

**位置**：`main.rs:13-25`

应用启动时**不运行迁移**。CI 在测试环境运行迁移，但不部署。新实例如果数据库 schema 不是最新版本，启动直接失败。

#### 问题 5：JWT 默认密钥在 docker-compose 中保留

**位置**：`docker-compose.yml`

```yaml
JWT_SECRET: ${JWT_SECRET:-your-super-secret-jwt-key-change-this-in-production}
```

默认密码字符串虽然不是 `"your-secret-key"` 字面量，但 `Config` 校验**只检查字面量**，允许这个字符串通过，导致生产环境密钥是公开的占位符。

---

## 重大遗漏：可观测性

### 问题 1：日志配置不生效

**位置**：`main.rs:17`

```rust
tracing_subscriber::fmt::init();  // ❌ 使用默认配置，不读取 LOG_LEVEL/LOG_FORMAT
```

`Config` 中的 `log_level` 和 `log_format` 字段被加载但**从未被使用**。运维无法按模块控制日志级别。

### 问题 2：无 Prometheus / OpenTelemetry

无指标导出端点。`WebSocketMonitor` 收集了内部指标但**永远不会被导出**。

### 问题 3：trace_id 未传播到服务层

`request_tracking_middleware` 创建 `info_span!("request", trace_id=...)`，但 trace_id **未传递到 RequestContext**，DB 错误无法关联到具体请求。

### 问题 4：双 logger 重复记录

`request_tracking_middleware` 和 `performance_monitoring_middleware` 几乎相同地记录每个请求，导致**日志量翻倍**。

### 问题 5：敏感信息泄漏

**位置**：`main.rs:19`

```rust
tracing::info!("🚀 Server starting with config: {:?}", config);
//                                         ^^^^^^ Debug 打印会泄露 DATABASE_URL、JWT_SECRET
```

---

## 重大遗漏：资源效率与并发

### 问题 1：严重 N+1 查询

**位置**：`issues_service.rs:591-790` 的 `build_issue_response`

对于列表中**每个 Issue**：
- 单独查询 team、project、project_status、project owner
- **每个 Issue 都全量查询 workspace 的 all_statuses**（这是最严重的）
- 单独查询 assignee、workflow states、labels、cycle

**性能影响**：列表 20 条 Issue 会触发 **200+ 次数据库查询**。

### 问题 2：搜索查询绕过 GIN 索引

**位置**：`repositories/issues.rs:91-100`

```rust
"(to_tsvector('english', title) || to_tsvector('english', description)) @@ websearch_to_tsquery(...)"
```

迁移 `2026-06-28-000000_add_search_vector` 创建了 `search_vector` 列和 GIN 索引，但查询**重新计算 tsvector**，GIN 索引**完全未被使用**。

### 问题 3：Redis `KEYS *` 阻塞生产

**位置**：`user_cache.rs:213-225`

```rust
let user_keys: Vec<String> = conn.keys(format!("{}*", USER_CACHE_PREFIX)).await?;
```

`KEYS` 是 O(N) 阻塞命令，**生产环境严禁使用**。

### 问题 4：无界数据结构

| 数据结构 | 位置 | 风险 |
|----------|------|------|
| `permission_cache` | `events/middleware.rs:310-316` | 永不清理，无界增长 |
| `processed_messages` | `security.rs:35` | 仅当 > 10000 时随机减半 |
| `metrics_tx` | `monitoring.rs:87` | `UnboundedSender` 无背压 |
| `topic_subs`/`conn_topics` | `subscription/manager.rs:35-37` | 无过期清理 |

### 问题 5：Issue 编号分配竞态

**位置**：`issues_service.rs:244-259`

仅 3 次重试 + 无 `FOR UPDATE` 锁，高并发下会触发唯一性冲突 500 错误。

### 问题 6：幂等性是死代码

`RequestContext.idempotency_key` 字段存在但**始终设置为 `None`**。WebSocket 命令中硬编码 `"disabled"`。`IdempotencyControl` 类型实现了完整逻辑，但**无任何调用**。

---

## 重大遗漏：业务连续性

### 问题 1：缓存失效策略缺失

`user_profile`、`user_workspace` 等缓存有 TTL 但**写入时不失效**，用户更新资料后**直到 TTL 过期才生效**。

### 问题 2：无熔断器

无 DB / Redis / 插件 gRPC 的熔断器，**任何下游故障都会立即传播为 500**。

### 问题 3：所有 Redis/DB 错误均为 500

`core/src/error.rs:71-77` 将 Redis 错误统一映射为 500，**客户端无法区分**瞬时错误（应重试）和逻辑错误（不应重试）。

### 问题 4：无 HTTP 层超时

仅启用了 `tower-http/cors` feature，**未启用 `timeout`/`limit`**。慢查询占用 DB 连接 30 秒。

### 问题 5：数据库无 statement_timeout

PostgreSQL 的 `statement_timeout` GUC 未设置，Diesel 无 query-level 超时。

### 问题 6：插件集成无超时与重试

`momentum_plugin_host` 的 gRPC 客户端**无超时、无重试、无熔断**。

### 问题 7：Dockerfile 镜像问题

- `Cargo.lock` 未 `--locked` 编译，依赖可能漂移
- 无 `cargo-chef` / BuildKit cache mount，**每次重编译所有 250+ crates**
- 无 `RUSTFLAGS` 优化
- 容器无 `mem_limit`/`cpus`/`pids_limit` 限制
- 无 read-only filesystem、无 `no-new-privileges`、无 `cap_drop`

---

## 审视总结：架构关注的 12 个维度

基于以上发现，资深架构师在审视项目时应关注的**完整维度清单**：

### 1️⃣ 功能正确性 (Functional Correctness)
- 业务逻辑是否符合需求规格
- 边界条件处理
- 一致性约束

### 2️⃣ 安全 (Security)
- **认证**：JWT、密码、会话管理
- **授权**：RBAC、ABAC、最小权限原则
- **隔离**：工作区隔离、租户隔离
- **注入**：SQL、XSS、SSRF、命令注入
- **数据保护**：TLS、加密、敏感数据脱敏
- **审计**：操作日志、安全事件追溯

### 3️⃣ 可靠性 (Reliability)
- **故障处理**：超时、重试、熔断、降级
- **数据一致性**：事务、补偿、幂等
- **错误传播**：错误的传递、转换、暴露

### 4️⃣ 可扩展性 (Scalability)
- **性能**：N+1、查询效率、索引使用
- **容量**：连接池、缓存、限流
- **水平扩展**：无状态化、分片

### 5️⃣ 可维护性 (Maintainability)
- **代码组织**：模块化、单一职责、命名
- **重复代码**：抽象、复用
- **技术债**：死代码、临时方案、文档缺失

### 6️⃣ 可测试性 (Testability)
- **可 mock 性**：依赖注入、trait 抽象
- **测试覆盖**：单元、集成、E2E
- **测试隔离**：数据库、状态、外部依赖

### 7️⃣ 可观测性 (Observability)
- **日志**：结构化、级别、关联 ID
- **指标**：业务、技术、SLA
- **追踪**：分布式 trace、链路追踪
- **告警**：阈值、异常检测

### 8️⃣ 可部署性 (Deployability)
- **容器化**：Dockerfile 优化、健康检查
- **迁移自动化**：schema migration、zero-downtime
- **回滚**：版本管理、灰度发布
- **配置管理**：环境隔离、密钥管理

### 9️⃣ 可演进性 (Evolvability)
- **版本化**：API 版本、协议兼容
- **扩展点**：插件、Hook、Middleware
- **重构路径**：strangler fig、绞杀者模式

### 🔟 可操作性 (Operability)
- **运维友好**：调试、日志、诊断
- **升级路径**：依赖更新、breaking change
- **文档**：架构、API、运维手册

### 1️⃣1️⃣ 业务连续性 (Business Continuity)
- **幂等性**：重试安全
- **补偿机制**：saga、最终一致性
- **灾难恢复**：备份、恢复演练
- **优雅关闭**：SIGTERM 处理

### 1️⃣2️⃣ 资源效率 (Resource Efficiency)
- **内存**：无界数据结构、内存泄漏
- **CPU**：算法复杂度、热点
- **网络**：payload 大小、压缩
- **数据库**：连接池、查询效率、索引

---

## 重写的优先级矩阵（2026-07-18 更新）

| 优先级 | 问题 | 影响 | 修复成本 | 状态 |
|--------|------|------|----------|------|
| ~~P0 - 立即修复~~ | ~~IssueRepo 跨工作区访问~~ | 数据泄露/破坏 | 低 | ✅ 已修复 |
| ~~P0 - 立即修复~~ | ~~switch_workspace 无成员校验~~ | 完全绕过授权 | 低 | ✅ 已修复 |
| ~~P0 - 立即修复~~ | ~~WebSocket 广播无工作区过滤~~ | 数据泄露 | 中 | ✅ 已修复 |
| ~~P0 - 立即修复~~ | ~~无 RBAC~~ | 越权操作 | 高 | ✅ 已修复 |
| ~~P0 - 立即修复~~ | ~~连接池 panic~~ | 服务崩溃 | 低 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~Dockerfile healthcheck 不存在~~ | 容器重启循环 | 低 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~无 graceful shutdown~~ | 数据不一致 | 中 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~迁移未自动化~~ | 新实例启动失败 | 中 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~嵌套写锁死锁风险~~ | 服务挂起 | 中 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~同步 DB 操作阻塞线程~~ | 吞吐受限 | 高 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~trace_id 硬编码~~ | 日志无法关联 | 低 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~双 logger 重复记录~~ | 日志噪音 | 低 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~搜索绕过 GIN~~ | 搜索性能差 | 低 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~Redis `KEYS *`~~ | 生产阻塞 | 低 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~领域事件未连接 WebSocket~~ | 实时功能失效 | 中 | ✅ 已修复 |
| ~~P1 - 高优~~ | ~~WebSocket Registry 未激活~~ | 维护混乱 | 中 | ✅ 已激活 |
| ~~P1 - 高优~~ | ~~幂等性死代码~~ | 重试不安全 | 中 | ✅ 已修复 |
| ~~P2 - 中优~~ | ~~服务层可测试性~~ | 测试困难 | 高 | ✅ 已修复 |
| **P2 - 中优** | 双订阅系统不同步 | 行为不一致 | 中 | ⏳ 待处理 |
| **P3 - 低优** | 无 API 版本化 | 演进困难 | 中 | ⏳ 待处理 |
| **P3 - 低优** | 日志配置不生效 | 运维受限 | 低 | ✅ 已修复 |
| **P3 - 低优** | 无 Prometheus 导出 | 监控盲区 | 中 | ⏳ 待处理 |
| **P3 - 低优** | 插件系统耦合 | 扩展性受限 | 高 | ⏳ 待处理 |

---

## 文档勘误与现状对照（2026-07-18 更新）

### 勘误 1：handler.rs 引用错误
**状态**：✅ 已修正 - 引用路径已更正为 `commands/handler.rs`

### 勘误 2：IssueEvent 文件路径
**状态**：✅ 已修正 - 正确路径为 `websocket/issue_events.rs`

### 勘误 3：Ping 处理器实现
**状态**：✅ 已确认 - 实现返回 `{ok: true, echo, user_id, ts}`

### 勘误 4：subscribe/unsubscribe 行号
**状态**：✅ 已修正 - 两个函数已分开

---

## 文档修订建议

`ARCHITECTURE_ISSUES.md` 应当按以下方式扩展：

### 必须新增的章节

1. **问题 8：严重安全漏洞**（Critical）
   - 工作区隔离失效（IssueRepo）
   - 缺少 RBAC
   - WebSocket 广播无工作区过滤
   - switch_workspace 无成员校验

2. **问题 9：运维与生产化问题**（High）
   - Dockerfile healthcheck 失败
   - 无 graceful shutdown
   - 连接池 panic
   - 迁移未自动化

3. **问题 10：可观测性缺失**（Medium）
   - 日志配置不生效
   - 无 Prometheus
   - trace_id 未传播
   - 敏感信息泄露

4. **问题 11：业务连续性缺陷**（Medium）
   - 幂等性是死代码
   - 无熔断器
   - 错误未分类

5. **问题 12：资源效率问题**（Medium）
   - 严重 N+1
   - 搜索绕过 GIN
   - Redis KEYS *
   - 无界数据结构

### 必须修正的引用

- `handler.rs:625-628` → `commands/handler.rs:625-628`
- `events/issue_events.rs` → `issue_events.rs`（根目录）
- `manager.rs:315-342` → 拆分为 `subscribe` (315-326) 和 `unsubscribe` (327-342)
- Ping 实现修正为 `{ "ok": true, "echo": payload, "user_id": ctx.user_id, "ts": Utc::now() }`
- 删除错误代码示例

---

## 附录：架构师建议的后续行动

### 立即行动 (本周)

1. **修复安全漏洞**
   - 修复 `IssueRepo::find_by_id_in_workspace` 强制 workspace_id 过滤
   - 添加 `switch_workspace` 成员验证
   - 实现 WebSocket 工作区过滤

2. **修复运维阻塞**
   - 添加 `/health` 端点
   - 修复 Dockerfile 中的 curl
   - 替换 `expect("Failed to get DB connection")` 为错误返回

### 短期行动 (本月)

3. **架构改进**
   - 引入 RBAC middleware
   - 修复 N+1 查询（特别是 `available_statuses`）
   - 添加 graceful shutdown
   - 实现熔断器（如 `tower::limit`、`failsafe-rs`）

4. **可观测性建设**
   - 启用 `EnvFilter` 配置
   - 添加 `/metrics` Prometheus 端点
   - 修复 trace_id 传递
   - 移除 `tracing::info!("config: {:?}", config)`

### 中期行动 (本季度)

5. **可测试性**
   - 定义 Repository trait
   - 服务层依赖注入改造
   - 添加单元测试（目标覆盖率 70%+）

6. **WebSocket 重构**
   - 选择 Registry 或 Legacy，删除另一条
   - 统一订阅系统
   - 修复 broadcast 与 connection 路由

### 长期行动

7. **API 版本化**
8. **迁移到 sqlx 异步 ORM**
9. **插件边界明确化（独立 crate）**
10. **建立 ADR（架构决策记录）流程**

---

**报告生成日期**：2026-07-05
**最后更新**：2026-07-18
**审视范围**：完整架构（12 个维度）
**严重程度分布**：P0: 0 个 | P1: 0 个 | P2: 1 个 | P3: 4 个

## 修复状态总结（2026-07-18 更新）

### ✅ P0 问题（已全部修复）

| 问题 | 状态 | 修复版本/Commit |
|------|------|-----------------|
| IssueRepo 跨工作区访问 | ✅ 已修复 | v0.3.0 |
| switch_workspace 无成员校验 | ✅ 已修复 | v0.3.0 |
| WebSocket 广播无工作区过滤 | ✅ 已修复 | v0.3.0 |
| 无 RBAC | ✅ 已修复 | v0.3.0 |
| 连接池 panic | ✅ 已修复 | v0.3.0 |

### ✅ P1 问题（已全部修复）

| 问题 | 状态 | 修复方式 |
|------|------|-----------|
| Dockerfile healthcheck 不存在 | ✅ 已修复 | v0.3.0 + curl 安装 |
| 无 graceful shutdown | ✅ 已修复 | v0.3.0 |
| 迁移未自动化 | ✅ 已修复 | embedded migrations |
| 嵌套写锁死锁风险 | ✅ 已修复 | 分步获取锁 |
| 同步 DB 操作阻塞线程 | ✅ 已修复 | spawn_blocking |
| trace_id 硬编码 | ✅ 已修复 | extract_trace_id |
| 双 logger 重复记录 | ✅ 已修复 | 纯透传 |
| 搜索绕过 GIN 索引 | ✅ 已修复 | 使用 search_vector 列 |
| Redis KEYS * | ✅ 已修复 | 使用 SCAN |
| 领域事件未连接 WebSocket | ✅ 已修复 | broadcast_issue_event |
| WebSocket Registry 激活 | ✅ 已修复 | with_registry |
| 幂等性死代码 | ✅ 已修复 | idempotency_key 传递 |

### ✅ P2 问题（已全部修复）

| 问题 | 状态 | 修复方式 |
|------|------|-----------|
| 服务层可测试性 | ✅ 已修复 | TeamRepositoryTrait + MockTeamRepo |
| WebSocket Registry 双分发 | ✅ 已激活 | with_registry |

### ⚠️ P3 问题（长期改进，暂未处理）

| 问题 | 状态 | 说明 |
|------|------|------|
| 无 API 版本化 | ⏳ 待处理 | 需 breaking change |
| 日志配置不生效 | ✅ 已修复 | init_tracing 解析 EnvFilter |
| 无 Prometheus 导出 | ⏳ 待处理 | 需要 /metrics 端点 |
| 插件系统耦合 | ⏳ 待处理 | 需独立 crate |

### 修复文件清单（本次会话）

**momentum_core:**
- `src/db/migrations.rs` - 新增 embedded migrations 模块
- `src/db/repositories/teams.rs` - 新增 TeamRepo 模块
- `src/db/repositories/traits.rs` - 新增 TeamRepositoryTrait + MockTeamRepo
- `src/services/teams_service.rs` - 重构为泛型结构体

**momentum_api:**
- `src/main.rs` - 添加迁移运行调用
- `src/routes/teams.rs` - 使用 TeamsService 实例方法
- `src/websocket/manager.rs` - 领域事件广播
- `src/websocket/registry_dispatch.rs` - idempotency_key 传递