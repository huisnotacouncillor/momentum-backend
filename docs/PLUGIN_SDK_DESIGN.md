# Momentum Plugin SDK 设计文档

> **Phase 1：单进程验证 / Phase 2：gRPC 独立进程**
> 文档版本：v0.1 · 2026-06-19

**相关文档**：
- [产品规划 v3.0](./PRODUCT_PLAN.md)
- [架构设计 v1.0](./ARCHITECTURE.md)

---

## 0. 设计目标

| 目标 | 说明 |
|------|------|
| **隔离** | 插件崩溃不影响核心；资源/权限受限 |
| **可扩展** | 任意能力通过扩展点接入，核心数据模型不变 |
| **可演进** | 单进程（Phase 1）→ gRPC（Phase 2）切换不改业务代码 |
| **开发友好** | Rust + YAML；一个 internal dummy 插件 1 天写完 |

---

## 1. 总体架构

### 1.1 进程模型

**Phase 1（当前，本仓库内）**：插件编译为**独立 Rust binary**，核心通过 gRPC（`tonic`）与其通信。

```
┌────────────────────────────────────────────────────────────┐
│  momentum-core (单 binary，单进程内)                       │
│  ┌────────────────────────────────────────────────┐       │
│  │ HTTP Routes → Services → Repositories → PG     │       │
│  └────────────────────────────────────────────────┘       │
│                         ↑                                  │
│  ┌──────────────────────┴─────────────────────────┐       │
│  │ Plugin Host (gRPC server)                      │       │
│  │  · 进程管理（spawn / restart / kill）           │       │
│  │  · 权限网关                                     │       │
│  │  · 调用分发（plugin_id + method → channel）    │       │
│  └──────────────────────┬─────────────────────────┘       │
└─────────────────────────┼──────────────────────────────────┘
                          │ gRPC (tonic over UDS or TCP)
       ┌──────────────────┼──────────────────┐
       │                  │                  │
   ┌───▼────┐         ┌───▼────┐         ┌───▼────┐
   │dummy   │         │ei      │         │...     │
   │plugin  │         │plugin  │         │        │
   │(binary)│         │(binary)│         │        │
   └────────┘         └────────┘         └────────┘
```

**为什么 gRPC 而不是 cdylib + dlopen**：
- ✅ 进程隔离（崩溃不影响核心）
- ✅ 多语言（未来 Python/Go 插件可零成本接入）
- ✅ 独立部署 / 独立升级
- ✅ 显式的 schema 契约（proto 文件）
- ⚠️ 比 FFI 慢一点（但 gRPC 足够快：~1ms 本地调用）

### 1.2 当前阶段约束

- **Phase 1**：插件 host 和 core 都在 `momentum-backend` 仓库内，但**作为独立 binary 编译运行**（`cargo run --bin plugin-dummy`）
- **gRPC channel**：用 **Unix Domain Socket**（`/tmp/momentum-plugins/{plugin_id}.sock`），低延迟、无网络配置
- **协议**：`proto/plugin.proto` 是单一真相源，Rust 类型用 `tonic-build` 自动生成

---

## 2. Manifest 规范

**完整字段**（v0.1 版本）：

```yaml
# plugin.yaml
apiVersion: v1                              # manifest schema 版本
kind: Plugin

# === 标识 ===
id: embodied-intelligence                  # 全局唯一，反向 DNS 风格
name: Embodied Intelligence Pack
version: 0.1.0
publisher: Momentum
description: 软硬件一体研发工作流
license: commercial
homepage: https://momentum.so/plugins/ei

# === 兼容 ===
core_compat: ">=1.0.0,<2.0.0"              # semver 范围

# === 入口 ===
entrypoint:
  binary: ./bin/plugin-ei                  # 相对 plugin.yaml
  # 或者 container: image:tag（未来）

# === 8 大扩展点（v0.1 实现子集） ===
extensions:
  fields:                                  # 扩展点 1
    - key: issue.effort
      type: number
      label: Effort (hours)
      required: false
      min: 0
      max: 1000

  # 扩展点 2-8 在 v0.1 暂以 stub 形式预留
  artifact_types: []
  workflows: []
  agents:
    - id: dummy-agent                       # v0.1 验证用
      description: A dummy agent for testing
      input_schema:
        type: object
        properties:
          message:
            type: string
      output_schema:
        type: object
        properties:
          reply:
            type: string
  views: []
  integrations: []
  webhooks:
    subscribes: [issue.created]
    publishes: [dummy.test]
  storage:
    namespaces: [telemetry]
    max_size_mb: 100

# === 权限申请（白名单） ===
permissions:
  - issue.read
  - issue.write
  - issue.field.read:issue.effort
  - issue.field.write:issue.effort
  - agent.invoke:dummy-agent
  - storage.read:telemetry
  - storage.write:telemetry
  - event.subscribe:issue.created
  - event.publish:dummy.test
```

---

## 3. 扩展点 v0.1 实现范围

| # | 扩展点 | v0.1 状态 | 说明 |
|---|--------|----------|------|
| 1 | **Field Extension** | ✅ 完整 | 注册自定义字段；写值；按字段过滤 |
| 2 | View Extension | 🔜 stub | 暂只占位 |
| 3 | Agent SDK | ✅ 简化版 | 注册 Agent 类型 + 同步 invoke（流式 v0.2）|
| 4 | Workflow Extension | 🔜 stub | |
| 5 | Integration Hooks | 🔜 stub | |
| 6 | Webhook Bus | ✅ 简化版 | 订阅 issue.created；发布自定义事件 |
| 7 | Storage Namespace | ✅ 完整 | KV 存储，namespace 隔离 |
| 8 | Permission Hooks | ✅ 完整 | 申请 + 强制检查 |

**v0.1 验证场景**：
- Dummy 插件注册 `issue.effort` 字段
- 创建 issue 时可写 effort
- 读 issue 时带 effort
- 触发 dummy-agent，返回 mock 回复
- 写 storage.telemetry，跨调用可读

---

## 4. 数据库模型

**6 张新表**（写在下个 migration）：

```sql
-- 字段定义（插件注册）
CREATE TABLE issue_field_definitions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL,                  -- 引用 plugins.id
    field_key TEXT NOT NULL,                  -- 如 'effort'
    label TEXT NOT NULL,
    field_type TEXT NOT NULL,                 -- 'text' | 'number' | 'enum' | 'date' | 'user' | 'bool'
    options JSONB,                            -- enum 选项 / 校验规则
    required BOOLEAN NOT NULL DEFAULT false,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(workspace_id, plugin_id, field_key)
);

-- 字段值
CREATE TABLE issue_field_values (
    issue_id UUID NOT NULL REFERENCES issues(id) ON DELETE CASCADE,
    field_id UUID NOT NULL REFERENCES issue_field_definitions(id) ON DELETE CASCADE,
    value JSONB NOT NULL,                     -- 实际值
    text_value TEXT,                          -- 反范式（搜索/索引用）
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (issue_id, field_id)
);
CREATE INDEX idx_issue_field_values_text ON issue_field_values(text_value);

-- 插件元数据
CREATE TABLE plugins (
    id TEXT PRIMARY KEY,                      -- 'embodied-intelligence'
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    publisher TEXT,
    manifest JSONB NOT NULL,                  -- 完整 manifest
    status TEXT NOT NULL DEFAULT 'available', -- 'available' | 'deprecated'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 插件安装
CREATE TABLE plugin_installations (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    workspace_id UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    plugin_id TEXT NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    config JSONB NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'installed', -- 'installed' | 'enabled' | 'disabled' | 'error'
    installed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    enabled_at TIMESTAMPTZ,
    error_message TEXT,
    UNIQUE(workspace_id, plugin_id)
);

-- 插件存储
CREATE TABLE plugin_storage (
    plugin_id TEXT NOT NULL,
    workspace_id UUID NOT NULL,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (plugin_id, workspace_id, namespace, key)
);

-- 插件审计
CREATE TABLE plugin_audit (
    id BIGSERIAL PRIMARY KEY,
    plugin_id TEXT NOT NULL,
    workspace_id UUID,
    event TEXT NOT NULL,                      -- 'installed' | 'enabled' | 'field.set' | 'agent.invoked' | ...
    payload JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_plugin_audit_plugin ON plugin_audit(plugin_id, created_at DESC);
```

**Issue 表改动**：

```sql
ALTER TABLE issues ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
```

（version 字段给乐观锁用，event 同步用）

---

## 5. gRPC 协议（proto/plugin.proto）

**v0.1 服务定义**：

```protobuf
syntax = "proto3";
package momentum.plugin.v1;

// Plugin ↔ Core 双向流
service PluginService {
  // === 生命周期 ===
  rpc Handshake(HandshakeRequest) returns (HandshakeResponse);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  
  // === 字段（核心 → 插件） ===
  rpc OnFieldWrite(OnFieldWriteRequest) returns (OnFieldWriteResponse);
  
  // === Agent（核心 → 插件） ===
  rpc InvokeAgent(InvokeAgentRequest) returns (InvokeAgentResponse);
  
  // === 存储（核心 → 插件） ===
  rpc StorageGet(StorageGetRequest) returns (StorageGetResponse);
  rpc StoragePut(StoragePutRequest) returns (StoragePutResponse);
  
  // === 事件（核心 → 插件） ===
  rpc SubscribeEvents(stream EventEnvelope) returns (Empty);
  rpc PublishEvent(PublishEventRequest) returns (Empty);
}

// ============ Messages ============

message HandshakeRequest {
  string plugin_id = 1;
  string plugin_version = 2;
  string core_version = 3;
  string workspace_id = 4;     // 插件实例绑定到一个 workspace
  string auth_token = 5;       // Core 颁发的短时 token
}

message HandshakeResponse {
  bool ok = 1;
  string error = 2;
  // 插件注册时声明的所有扩展点，Core 确认接收
  repeated ExtensionDefinition extensions = 3;
}

message ExtensionDefinition {
  string type = 1;              // "field" | "agent" | ...
  string key = 2;
  google.protobuf.Struct config = 3;
}

message HeartbeatRequest {
  int64 ts_ms = 1;
}
message HeartbeatResponse {
  int64 ts_ms = 1;
  int64 server_ts_ms = 2;
}

// 字段写入
message OnFieldWriteRequest {
  string workspace_id = 1;
  string issue_id = 2;
  string field_key = 3;
  google.protobuf.Value new_value = 4;
  google.protobuf.Value old_value = 5;
  string actor_id = 6;
}
message OnFieldWriteResponse {
  bool ok = 1;
  string error = 2;
  // 插件可拒绝写入（如校验失败）
  google.protobuf.Value rejected_value = 3;
}

// Agent 调用
message InvokeAgentRequest {
  string workspace_id = 1;
  string issue_id = 2;
  string agent_id = 3;
  google.protobuf.Struct input = 4;
  string invocation_id = 5;
}
message InvokeAgentResponse {
  bool ok = 1;
  string error = 2;
  google.protobuf.Struct output = 3;
  int32 tokens_in = 4;
  int32 tokens_out = 5;
  int32 duration_ms = 6;
}

// 存储
message StorageGetRequest {
  string workspace_id = 1;
  string namespace = 2;
  string key = 3;
}
message StorageGetResponse {
  bool found = 1;
  google.protobuf.Value value = 2;
}
message StoragePutRequest {
  string workspace_id = 1;
  string namespace = 2;
  string key = 3;
  google.protobuf.Value value = 4;
  int64 ttl_seconds = 5;     // 0 = 永久
}
message StoragePutResponse {
  bool ok = 1;
}

// 事件
message EventEnvelope {
  int64 id = 1;               // outbox id
  string workspace_id = 2;
  string event_type = 3;
  google.protobuf.Struct payload = 4;
  int64 occurred_at_ms = 5;
}

message PublishEventRequest {
  string workspace_id = 1;
  string event_type = 2;      // 必须先在 manifest.permissions 申请
  google.protobuf.Struct payload = 3;
}

message Empty {}
```

---

## 6. Core 端模块结构

```
src/
├── plugins/                              # Plugin SDK + 内部实现
│   ├── mod.rs                            # 模块入口
│   ├── manifest.rs                       # Manifest 解析 + 验证
│   ├── error.rs                          # 错误类型
│   ├── extension/
│   │   ├── mod.rs
│   │   ├── field.rs                      # Field 扩展点核心实现
│   │   ├── agent.rs                      # Agent 扩展点核心实现
│   │   ├── storage.rs                    # Storage 扩展点核心实现
│   │   └── event.rs                      # Event 扩展点核心实现
│   ├── registry/
│   │   ├── mod.rs
│   │   ├── state.rs                      # 内存状态
│   │   ├── db.rs                         # DB 持久化
│   │   └── lifecycle.rs                  # install/enable/disable/uninstall
│   ├── permission.rs                     # 权限校验
│   └── audit.rs                          # 审计日志
│
├── plugin_host/                          # gRPC 插件 host
│   ├── mod.rs
│   ├── server.rs                         # gRPC server（接收插件调用）
│   ├── client.rs                         # gRPC client（Core → 插件）
│   ├── process.rs                        # 进程管理
│   └── supervisor.rs                     # 重启 / 健康检查
│
├── db/
│   ├── models/
│   │   ├── issue_field_definition.rs     # 新增
│   │   ├── issue_field_value.rs          # 新增
│   │   ├── plugin.rs                     # 新增
│   │   ├── plugin_installation.rs        # 新增
│   │   └── plugin_audit.rs               # 新增
│   └── repositories/
│       ├── issue_field_definitions.rs    # 新增
│       ├── issue_field_values.rs         # 新增
│       ├── plugins.rs                    # 新增
│       ├── plugin_installations.rs       # 新增
│       └── plugin_storage.rs             # 新增
│
└── services/
    ├── issues_service.rs                 # 改动：读写时带 field_values
    └── plugin_service.rs                 # 新增：插件管理的业务用例
```

**examples/dummy-plugin/** (独立 crate)：

```
examples/dummy-plugin/
├── Cargo.toml                            # [[bin]] name = "plugin-dummy"
├── plugin.yaml                           # manifest
├── src/
│   └── main.rs                           # 启动 gRPC server
```

---

## 7. 关键流程

### 7.1 插件安装

```
POST /api/v1/plugins/install { plugin_id, config }
  ↓
PluginService::install()
  ├── 1. 验证 manifest
  ├── 2. INSERT plugins (idempotent by id+version)
  ├── 3. INSERT plugin_installations
  ├── 4. 加载 plugin.yaml 提取 extensions
  ├── 5. 写 issue_field_definitions
  ├── 6. 写 plugin_audit ('installed')
  └── 7. 返回 installation_id
```

### 7.2 插件启用

```
POST /api/v1/plugins/{installation_id}/enable
  ↓
PluginService::enable()
  ├── 1. 状态检查：installed → enabled
  ├── 2. 启动子进程：
  │     spawn process: ./bin/plugin-dummy
  │     set env: WORKSPACE_ID, AUTH_TOKEN, SOCKET_PATH
  │     wait for UDS file
  ├── 3. gRPC Handshake（验证 plugin_id / version / token）
  ├── 4. 把插件声明的 extensions 同步到 issue_field_definitions / agent_runs 路由表
  ├── 5. 写 plugin_audit ('enabled')
  └── 6. 启动 Heartbeat 监控 + 自动重启 supervisor
```

### 7.3 字段读取

```
GET /api/v1/issues/{id}
  ↓
IssueService::get(id)
  ├── 1. SELECT * FROM issues WHERE id = ?
  ├── 2. SELECT field_id, value, field_key
  │     FROM issue_field_values v
  │     JOIN issue_field_definitions d ON v.field_id = d.id
  │     WHERE v.issue_id = ?
  ├── 3. 组装 response: { ...issue, field_values: { effort: 8, ... } }
  └── 4. 缓存到 Redis（5min TTL）
```

### 7.4 字段写入（含插件校验）

```
PATCH /api/v1/issues/{id} { ..., field_values: { effort: 8 } }
  ↓
IssueService::update(id, patch)
  ├── 1. BEGIN TRANSACTION
  ├── 2. UPDATE issues SET ... (基本字段)
  ├── 3. 对 patch.field_values 每个字段：
  │     a. 检查 field 存在
  │     b. 检查核心权限
  │     c. 如果插件在线：gRPC OnFieldWrite → 插件可拒绝
  │     d. UPSERT issue_field_values
  │     e. 更新 text_value（用于搜索）
  ├── 4. INSERT outbox event
  ├── 5. COMMIT
  └── 6. 返回更新后的 issue
```

### 7.5 Agent 调用

```
POST /api/v1/agents/{agent_id}/runs
  ↓
AgentRunner::invoke(agent_id, input)
  ├── 1. 查 agent_id 对应哪个插件
  ├── 2. gRPC InvokeAgent → 插件
  ├── 3. 插件执行（可能调 LLM、查 DB、调用 MCP）
  ├── 4. 写 agent_runs + agent_steps
  ├── 5. 返回 output
```

### 7.6 事件订阅

```
插件启动时 SubscribeEvents(workspace_id)
  ↓
Core 持续 stream EventEnvelope（来自 outbox）
  ↓
插件收到后处理：
  · issue.created → 触发某个本地逻辑
  · 写 plugin_audit
  · 必要时 PublishEvent
```

---

## 8. 权限模型

**三级检查**：

| 层级 | 检查点 | 失败行为 |
|------|--------|---------|
| **HTTP** | 路由层中间件 | 401/403 |
| **Service** | 业务用例 | AppError::Forbidden |
| **Plugin 调用** | Plugin Host 权限网关 | gRPC PermissionDenied + 审计 |

**Plugin 权限检查流程**：

```rust
// Plugin 调用 → Host 收到
fn check_permission(
    plugin_id: &str,
    requested: &Permission,    // 如 "issue.field.write:issue.effort"
    manifest: &Manifest,
) -> Result<(), AppError> {
    let granted = &manifest.permissions;
    if granted.iter().any(|p| permission_matches(p, requested)) {
        Ok(())
    } else {
        plugin_audit(plugin_id, "permission_denied", requested);
        Err(AppError::Forbidden(format!(
            "Plugin {} not granted permission {}",
            plugin_id, requested
        )))
    }
}
```

**权限匹配规则**：
- 精确匹配：`issue.read` 包含 `issue.read`
- 资源范围：`issue.field.write:issue.effort` 要求 `issue.field.write` 范围 + `key == effort`
- 通配符（v0.2）：`issue.field.write:*` 允许写所有字段

---

## 9. 错误处理

```rust
// src/plugins/error.rs
#[derive(thiserror::Error, Debug)]
pub enum PluginError {
    #[error("manifest invalid: {0}")]
    ManifestInvalid(String),
    
    #[error("plugin not found: {0}")]
    NotFound(String),
    
    #[error("plugin already installed: {0}")]
    AlreadyInstalled(String),
    
    #[error("plugin not enabled: {0}")]
    NotEnabled(String),
    
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("process spawn failed: {0}")]
    ProcessSpawn(String),
    
    #[error("handshake failed: {0}")]
    Handshake(String),
    
    #[error("gRPC call failed: {0}")]
    GrpcCall(#[from] tonic::Status),
    
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
}
```

**gRPC 错误映射**：
- `PermissionDenied` → `tonic::Status::permission_denied`
- `ProcessSpawn` → `tonic::Status::unavailable`
- `Handshake` → `tonic::Status::failed_precondition`

---

## 10. 测试策略

**v0.1 测试范围**：

| 测试 | 类型 | 验证内容 |
|------|------|---------|
| Manifest 解析 | unit | 合法 manifest → 解析成功；非法 → 错误 |
| 权限匹配 | unit | 各种 manifest.permissions 组合 |
| Field 写值 | integration | 安装 dummy → 创建 issue 带 effort → 读出 |
| Field 过滤 | integration | filter[effort]=8 → 返回匹配的 issue |
| Agent invoke | integration | 触发 dummy-agent → 返回 mock 输出 |
| Storage | integration | put → get → 跨调用读出 |
| 进程崩溃恢复 | manual | kill 插件进程 → 自动重启 |
| e2e | integration | 完整流程 |

---

## 11. 未来演进（v0.2+）

| v0.2 | v0.3 | v1.0 |
|------|------|------|
| 流式 Agent 输出 | Workflow 完整实现 | Marketplace |
| View 扩展（iframe） | Integration Hooks | 第三方插件签名 |
| 字段 enum / 校验 | 插件更新通知 | 插件计费 |
| Webhook 重试 | UI 插件市场 | 多 Region |
| 异步事件 | CRDT 协同编辑 | 插件隔离强化 |

---

**最后更新**：2026-06-19 · v0.1
