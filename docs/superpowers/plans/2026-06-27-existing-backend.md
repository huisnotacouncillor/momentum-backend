# 现有后端架构说明

> **面向 AI 代理的工作者：** 本文档描述**已有代码**的架构，不需要新建后端服务。
> 所有新增逻辑基于 **Rust / Axum**，不引入 NestJS/GraphQL/Node.js。

---

## 一、现有架构总览

```
momentum_backend/
├── momentum_core/          # Pure Rust，无 HTTP 依赖
│   ├── src/
│   │   ├── schema.rs      # Diesel schema (37 张表)
│   │   ├── db/
│   │   │   ├── models/    # 24 个模型
│   │   │   ├── repositories/ # 19 个 repo
│   │   │   └── enums/    # 枚举类型
│   │   ├── services/     # 15 个 service
│   │   └── plugins/      # Plugin 系统 (Field/Agent/Storage/Event)
│   └── migrations/        # Diesel 迁移
│
├── momentum_api/           # Axum HTTP 层
│   └── src/
│       ├── routes/        # ~60 个 HTTP 端点
│       ├── websocket/      # WebSocket 基础设施
│       ├── middleware/     # JWT / Logger / Tracking
│       ├── cache/         # Redis 缓存
│       ├── state.rs       # AppState (含 Supervisor)
│       └── main.rs        # 服务器入口
│
├── momentum_plugin_host/   # gRPC client (Plugin 进程管理)
│   └── src/
│       ├── supervisor.rs  # 插件进程 Supervisor
│       ├── process.rs    # spawn/terminate
│       └── agent_impl.rs # gRPC invoke_agent client
│
└── plugins/plugin-dummy/  # 示例插件
```

---

## 二、现有 momentum_api 路由清单

### 2.1 Auth 路由 (`/auth`)
```
POST /auth/register
POST /auth/login
GET  /auth/profile
POST /auth/logout
POST /auth/switch-workspace
```

### 2.2 Workspace 路由 (`/workspaces`)
```
POST   /workspaces
GET    /workspaces/current
PUT    /workspaces/:id
DELETE /workspaces/:id
```

### 2.3 Workspace Members (`/workspace-members`)
```
GET /workspace-members
GET /workspace-member-and-invitations
GET /workspaces/:id/members
```

### 2.4 Teams (`/teams`)
```
GET/POST   /teams
GET/PUT/DELETE /teams/:id
POST /teams/:id/members
GET  /teams/:id/members
PUT/DELETE /teams/:id/members/:user_id
GET /user/teams
```

### 2.5 Issues (`/issues`)
```
POST/GET /issues
GET/PUT/DELETE /issues/:id
GET/POST /issues/:id/comments
```

### 2.6 Comments (`/comments`)
```
GET/PUT/DELETE /comments/:id
```

### 2.7 Projects (`/projects`)
```
GET/POST   /projects
PUT/DELETE /projects/:id
```

### 2.8 Cycles (`/cycles`)
```
POST/GET   /cycles
GET/PUT/DELETE /cycles/:id
GET /cycles/:id/stats
GET/POST/DELETE /cycles/:id/issues
POST /cycles/auto-update-status
```

### 2.9 Project Statuses (`/project-statuses`)
```
GET/POST   /project-statuses
GET/PUT/DELETE /project-statuses/:id
```

### 2.10 Workflows (`/workflows`)
```
GET/POST /workflows
GET/PUT/DELETE /workflows/:id
GET/POST /workflows/:id/states
PUT/POST /workflows/:id/states/:state_id
GET/POST /issues/:id/transitions
GET /teams/:id/workflows
POST /teams/:id/workflows
GET /teams/:id/workflows/default/states
POST /teams/:id/workflows/default/states
PUT/POST /teams/:id/workflows/default/states/:state_id
```

### 2.11 Labels (`/labels`)
```
GET/POST   /labels
PUT/DELETE /labels/:id
```

### 2.12 Invitations (`/invitations`)
```
POST/GET /invitations
GET /invitations/:id
POST /invitations/:id/accept
POST /invitations/:id/decline
POST /invitations/:id/revoke
```

### 2.13 Users (`/users`)
```
PUT /users/profile
```

### 2.14 Plugins (刚实现的 7 个端点)
```
GET  /plugins
POST /plugins/install
POST /plugins/:inst_id/enable
POST /plugins/:inst_id/disable
GET  /workspaces/:wid/plugins
DELETE /workspaces/:wid/plugins/:pid
GET  /workspaces/:wid/fields
```

---

## 三、WebSocket 基础设施（已实现）

位置：`momentum_api/src/websocket/`

| 文件 | 功能 |
|------|------|
| `handler.rs` | WebSocket 升级处理 |
| `manager.rs` | 连接管理器（`ConnectionManager`）|
| `commands/handler.rs` | 命令分发（issues/labels/projects/user/workspace_members/workspaces）|
| `events/` | 事件广播（business.rs/core.rs/handlers.rs/middleware.rs）|
| `auth.rs` | WS JWT 认证 |
| `rate_limiter.rs` | 限流 |
| `retry_timeout.rs` | 重试机制 |
| `monitoring.rs` | 连接监控 |
| `security.rs` | 安全检查 |
| `error_mapper.rs` | 错误映射 |

**已有的事件订阅格式**：
```rust
// 订阅 issue 变更
subscribe("issue:{workspace_id}:{issue_id}")
// 订阅 workspace 通知
subscribe("workspace:{workspace_id}:notifications")
```

---

## 四、Plugin 系统（已实现）

### 4.1 扩展点

```rust
// FieldService - 读写 issue 自定义字段
pub trait FieldService {
    fn get_field_value(issue_id: Uuid, field_key: &str) -> Result<Option<serde_json::Value>, PluginError>;
    fn set_field_value(...) -> Result<(), PluginError>;
}

// AgentService - 调用 Agent 并记录
pub trait AgentService {
    fn start_run(...) -> Result<Uuid, PluginError>;  // 返回 agent_run.id
    fn complete_run(...) -> Result<(), PluginError>;
    fn fail_run(...) -> Result<(), PluginError>;
}

// StorageService - 命名空间隔离 KV
pub trait StorageService {
    fn get(namespace: &str, key: &str) -> Result<Option<Vec<u8>>, PluginError>;
    fn put(namespace: &str, key: &str, value: Vec<u8>) -> Result<(), PluginError>;
}

// EventService - 发布事件到 outbox
pub trait EventService {
    fn publish(event: &str, payload: serde_json::Value) -> Result<(), PluginError>;
}
```

### 4.2 Manifest 字段扩展

```yaml
extensions:
  fields:
    - key: issue.effort      # 必须是 issue.* 前缀
      type: number
      label: "Effort (hours)"
      required: false
      sort_order: 100
```

---

## 五、现有 Agent 相关代码

### 5.1 agent_runs 表

```sql
agent_runs (
  id UUID PRIMARY KEY,
  issue_id UUID REFERENCES issues(id),
  agent_type VARCHAR(30),  -- agent id，如 "dummy-agent"
  status VARCHAR(20),       -- pending / running / completed / failed
  input JSONB,              -- Agent 输入参数
  output JSONB,              -- Agent 输出
  error TEXT,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ
)
```

### 5.2 agent_impl.rs (gRPC client)

```rust
// 位置：momentum_plugin_host/src/agent_impl.rs

pub async fn invoke_agent(
    socket_path: &str,
    agent_id: &str,
    input: serde_json::Value,
) -> Result<serde_json::Value, PluginError> {
    // 建立 gRPC Unix socket 连接
    let channel = connect_lazy(socket_path)?;
    let mut client = PluginServiceClient::new(channel);

    // 构建请求
    let req = InvokeAgentRequest {
        agent_id: agent_id.to_string(),
        workspace_id: "...".to_string(),
        input_json: serde_json::to_string(&input)?,
    };

    // 调用
    let resp = client.invoke_agent(Request::new(req)).await?;
    Ok(serde_json::from_str(&resp.into_inner().output_json)?)
}
```

---

## 六、后端扩展的正确路径

### ❌ 不要做的
- 不要引入 NestJS / GraphQL / Apollo
- 不要创建独立 Node.js Sync Service
- 不要在 `momentum_core` 里引入 HTTP 相关依赖
- 不要用 `tonic::include_proto!` 在 `momentum_core`（它是 server-side macro）

### ✅ 应该做的

**新增 API 端点** → 加到 `momentum_api/src/routes/`
```rust
// 示例：在 routes/ 下新增文件
pub mod devices;
pub mod artifacts;
pub mod model_versions;

// 注册到 mod.rs
pub mod devices;
// 在 create_router 里添加
.route("/devices", get(devices::list_devices))
.route("/devices/:id", get(devices::get_device))
.route("/devices/:id/telemetry", ws(devices::device_telemetry_ws))
```

**新增 Service** → 加到 `momentum_core/src/services/`
```rust
pub mod devices_service;

pub struct DevicesService;

impl DevicesService {
    pub fn list(conn: &mut PgConnection, ws_id: Uuid) -> Result<Vec<Device>, AppError> { ... }
    pub fn register(...) -> Result<Device, AppError> { ... }
}
```

**新增 Repository** → 加到 `momentum_core/src/db/repositories/`
```rust
pub mod devices;

pub struct DeviceRepo;

impl DeviceRepo {
    pub fn list_by_workspace(conn: &mut PgConnection, ws_id: Uuid) -> Result<Vec<Device>, diesel::result::Error> { ... }
}
```

**新增 Model** → 加到 `momentum_core/src/db/models/`
```rust
pub mod device;
```

**新增迁移** → `momentum_core/migrations/`
```bash
diesel migration generate add_devices_and_artifacts
```

---

## 七、现有 WebSocket 如何扩展

现有 WS 基础设施在 `momentum_api/src/websocket/`，新增订阅类型：

```rust
// 在 commands/ 新增 handler
mod devices;

pub async fn handle_device_telemetry(ws: &mut WebSocket, msg: ClientMessage) -> Result<(), WsError> {
    // 1. 解析 device_id
    let device_id = parse_device_id(&msg)?;
    // 2. 认证 + 权限检查
    authorize_device_access(ws.user_id, device_id)?;
    // 3. 订阅 Redis channel
    let channel = format!("device:{}", device_id);
    redis_subscribe(ws, &channel).await;
    Ok(())
}
```

---

**本文档说明了现有后端架构。后续计划文档基于此编写，不再重复已实现内容。**
