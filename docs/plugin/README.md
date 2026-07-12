# Plugin SDK 文档

> Momentum 的插件系统：把"Momentum 团队协作内核"开放为可扩展平台，第三方/垂直行业插件通过 gRPC + Manifest 接入。

---

## 📚 文档导航

| 文档 | 用途 | 何时读 |
|---|---|---|
| **[README.md](./README.md)**（本文件） | 实战指南：5 分钟跑通第一个插件 | 想马上动手时 |
| **[design.md](./design.md)** | 架构设计：gRPC 协议、8 大扩展点、权限模型 | 需要理解"为什么这样设计" |
| **[handover-2026-06-27.md](./handover-2026-06-27.md)** | 当初实现 P0 的交接笔记：技术决策、坑、验证命令 | 接手后续工作 / 排查疑难 |

---

## 🎯 状态速览（2026-07-12）

- ✅ Dummy 插件已跑通（`plugins/plugin-dummy/`，独立 workspace member）
- ✅ 8 大扩展点中 **Field / Agent / Storage / Webhook / Permission** 已实现；View / Workflow / Integration 为 stub
- ✅ gRPC 用 **TCP localhost**（端口默认 19991，UDS 在 v0.2 切换）
- ⚠️ plugin-dummy 的 E2E 测试 (`--ignored`) 尚未接入 CI
- ⚠️ `state.rs` plugin_host 字段待整合（handover §4 待办）

代码入口：
- 协议：`proto/plugin.proto`
- Core：`momentum_core/src/plugins/{manifest,permission,extension/,registry/}/`
- 路由：`momentum_api/src/routes/plugins.rs`
- Host：`momentum_plugin_host/src/{supervisor,process,agent_impl}.rs`

---

## 1. 5 分钟快速体验

### 1.1 前置条件

```bash
# PostgreSQL 已起（docker / 本地均可）
docker ps | grep postgres  # 应有 postgres 容器

# Redis 可选
docker ps | grep redis     # 应有 redis 容器
```

### 1.2 跑 migrations

```bash
cd momentum-backend
export DATABASE_URL=postgres://postgres:postgres@localhost:5434/rust-backend
diesel migration run
# 应看到 22+ migrations 跑完，包括 "2026-06-27-113645_create_plugin_system"
```

### 1.3 Build

```bash
cargo build --release --bin rust_backend
cargo build --release --bin plugin-dummy
# 产物：target/release/rust_backend (后端)
#      target/release/plugin-dummy (内部 dummy 插件)
```

### 1.4 启动后端

```bash
./target/release/rust_backend
# 监听 127.0.0.1:8000
```

### 1.5 装第一个插件

```bash
# 注册 + 登录获取 JWT（参考现有 /auth/register /auth/login）
TOKEN="eyJ..."  # 从 auth/login 拿

# 装 dummy 插件
curl -X POST http://127.0.0.1:8000/plugins/install \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"manifest_yaml\": \"$(cat examples/dummy-plugin/plugin.yaml | jq -Rs .)\"}"
# 返回 { installation_id, plugin_id, status }

# 启用插件（启动 dummy 进程）
curl -X POST http://127.0.0.1:8000/plugins/{installation_id}/enable \
  -H "Authorization: Bearer $TOKEN"

# 看效果
curl http://127.0.0.1:8000/workspaces/{wid}/plugins \
  -H "Authorization: Bearer $TOKEN"
# 列出已安装插件
```

### 1.6 验证扩展字段

```bash
# 创建 issue 时填扩展字段
curl -X POST http://127.0.0.1:8000/issues \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "team_id": "...",
    "title": "Test issue with effort",
    "field_values": {"effort": 8}
  }'

# 读 issue 看 field_values 是否返回
curl http://127.0.0.1:8000/issues/{issue_id} \
  -H "Authorization: Bearer $TOKEN"
# 响应包含 "field_values": {"effort": 8}
```

---

## 2. 核心概念

### 2.1 3 层架构

```
┌─────────────────────────────────────────────┐
│  Core (Rust)                  ← 你现在在改的  │
│  - Issue / Project / Cycle                 │
│  - gRPC client → 插件进程                    │
│  - Plugin Registry（内存 + DB）              │
└──────────────┬──────────────────────────────┘
               │ gRPC over Unix Domain Socket
┌──────────────▼──────────────────────────────┐
│  Plugin (独立 binary)        ← 你要写的       │
│  - 启动时 listen UDS                        │
│  - 接收 gRPC 调用                            │
│  - 调自己的逻辑 / 调 LLM / 写 storage        │
└─────────────────────────────────────────────┘
```

### 2.2 8 大扩展点

| # | 扩展点 | 用途 | v0.1 状态 |
|---|--------|------|---------|
| 1 | **Field** | 注册 Issue 自定义字段 | ✅ |
| 2 | View | 自定义 UI 视图组件 | 🔜 stub |
| 3 | **Agent** | 注册可被 Core 调用的 AI Agent | ✅ |
| 4 | Workflow | 自定义工作流触发器 | 🔜 stub |
| 5 | Integration | 注册 MCP Server | 🔜 stub |
| 6 | **Webhook** | 订阅/发布平台事件 | ✅ |
| 7 | **Storage** | 隔离的 KV/Blob 存储 | ✅ |
| 8 | **Permission** | 申请权限 + 自动校验 | ✅ |

### 2.3 Manifest（plugin.yaml）

```yaml
apiVersion: v1
kind: Plugin
id: my-plugin                # 全局唯一（小写 + 数字 + 点 + - + _）
name: My Plugin
version: 1.0.0
publisher: Your Company
core_compat: ">=0.1.0"      # 核心版本兼容范围

entrypoint:
  binary: ./bin/my-plugin

extensions:
  fields:
    - key: issue.priority
      type: enum                # text/number/enum/date/user/bool
      label: Priority
      options: [low, medium, high]

  agents:
    - id: summarize
      description: Summarize issue description

  webhooks:
    subscribes: [issue.created]   # 订阅核心事件
    publishes: [my-plugin.ready]  # 发布自定义事件

  storage:
    - namespace: cache
      max_size_mb: 100

permissions:
  - issue.read
  - issue.field.read:issue.priority
  - issue.field.write:issue.priority
  - agent.invoke:summarize
  - event.subscribe:issue.created
  - event.publish:my-plugin.ready
  - storage.read:cache
  - storage.write:cache
```

完整 Manifest 规范见 [`design.md §2`](./design.md)。

---

## 3. 写你的第一个真实插件

### 3.1 推荐结构

```
plugins/
├── my-plugin/
│   ├── plugin.yaml
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs           # 启动 + gRPC server
│   │   ├── service.rs        # 业务逻辑
│   │   └── llm.rs            # 调 LLM
│   └── README.md
```

### 3.2 Cargo.toml 模板

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "my-plugin"
path = "src/main.rs"

[dependencies]
tonic = "0.12"
prost = "0.13"
prost-types = "0.13"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
chrono = "0.4"
tracing = "0.1"
tracing-subscriber = "0.3"
async-trait = "0.1"
hyper = { version = "1", features = ["server"] }
hyper-util = { version = "0.1", features = ["tokio"] }
futures = "0.3"
```

### 3.3 src/main.rs 骨架

```rust
use tonic::{transport::Server, Request, Response, Status};

pub mod proto {
    tonic::include_proto!("momentum.plugin.v1");
}

use proto::{
    plugin_service_server::{PluginService, PluginServiceServer},
    HandshakeRequest, HandshakeResponse, HeartbeatRequest, HeartbeatResponse,
    OnFieldWriteRequest, OnFieldWriteResponse, InvokeAgentRequest, InvokeAgentResponse,
    StorageGetRequest, StorageGetResponse, StoragePutRequest, StoragePutResponse,
    PublishEventRequest, EventEnvelope, Empty, SubscribeEventsRequest,
};

#[derive(Default)]
pub struct MyPlugin {
    // 你的状态
}

type ResponseStream<T> = std::pin::Pin<Box<dyn futures::Stream<Item = Result<T, Status>> + Send>>;

#[tonic::async_trait]
impl PluginService for MyPlugin {
    async fn handshake(&self, req: Request<HandshakeRequest>) -> Result<Response<HandshakeResponse>, Status> {
        tracing::info!("handshake from core");
        Ok(Response::new(HandshakeResponse { ok: true, error: "".into(), extensions: vec![] }))
    }

    async fn heartbeat(&self, req: Request<HeartbeatRequest>) -> Result<Response<HeartbeatResponse>, Status> {
        Ok(Response::new(HeartbeatResponse {
            ts_ms: req.into_inner().ts_ms,
            server_ts_ms: chrono::Utc::now().timestamp_millis(),
        }))
    }

    async fn on_field_write(&self, req: Request<OnFieldWriteRequest>) -> Result<Response<OnFieldWriteResponse>, Status> {
        let req = req.into_inner();
        tracing::info!("field write: issue={} field={}", req.issue_id, req.field_key);
        // TODO: 校验逻辑
        Ok(Response::new(OnFieldWriteResponse { ok: true, error: "".into(), rejected_value: None }))
    }

    async fn invoke_agent(&self, req: Request<InvokeAgentRequest>) -> Result<Response<InvokeAgentResponse>, Status> {
        let req = req.into_inner();
        // TODO: 调 LLM
        Ok(Response::new(InvokeAgentResponse {
            ok: true,
            error: "".into(),
            output: None,
            tokens_in: 0,
            tokens_out: 0,
            duration_ms: 0,
        }))
    }

    async fn storage_get(&self, _req: Request<StorageGetRequest>) -> Result<Response<StorageGetResponse>, Status> {
        Ok(Response::new(StorageGetResponse { found: false, value: None }))
    }

    async fn storage_put(&self, _req: Request<StoragePutRequest>) -> Result<Response<StoragePutResponse>, Status> {
        Ok(Response::new(StoragePutResponse { ok: true }))
    }

    type SubscribeEventsStream = ResponseStream<EventEnvelope>;
    async fn subscribe_events(&self, _req: Request<SubscribeEventsRequest>) -> Result<Response<Self::SubscribeEventsStream>, Status> {
        Ok(Response::new(Box::pin(futures::stream::empty())))
    }

    async fn publish_event(&self, req: Request<PublishEventRequest>) -> Result<Response<Empty>, Status> {
        tracing::info!("event publish: {}", req.into_inner().event_type);
        Ok(Response::new(Empty {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let socket = std::env::var("MOMENTUM_SOCKET_PATH")
        .unwrap_or_else(|_| "/tmp/my-plugin.sock".to_string());
    let _ = std::fs::remove_file(&socket);

    let addr: std::net::SocketAddr = format!("unix://{}", socket).parse()?;
    tracing::info!("my-plugin listening on {}", addr);

    Server::builder()
        .add_service(PluginServiceServer::new(MyPlugin::default()))
        .serve(addr)
        .await?;

    Ok(())
}
```

### 3.4 编译

```bash
# 把 .proto 拷过来
mkdir -p proto
cp ../momentum-backend/proto/plugin.proto proto/

# build.rs 生成 Rust 代码
cat > build.rs <<'EOF'
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile(&["proto/plugin.proto"], &["proto"])?;
    Ok(())
}
EOF
```

### 3.5 安装 + 启用

```bash
# 后端在跑 + 有 JWT
TOKEN="..."

# 装
curl -X POST http://127.0.0.1:8000/plugins/install \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d @- <<EOF
{
  "manifest_yaml": $(jq -Rs . < plugin.yaml)
}
EOF

# 启用（后端会 spawn 你的 binary）
curl -X POST http://127.0.0.1:8000/plugins/{installation_id}/enable \
  -H "Authorization: Bearer $TOKEN"
```

---

## 4. 调试技巧

### 4.1 看插件进程日志

```bash
# 后端 stdout 会打印插件进程的 stdout/stderr
# 启用插件时观察 backend 输出
```

### 4.2 手动启动插件测试

```bash
# 启动后，plugin-dummy 会 listen UDS
MOMENTUM_SOCKET_PATH=/tmp/test.sock \
MOMENTUM_PLUGIN_ID=dummy-plugin \
MOMENTUM_WORKSPACE_ID=00000000-0000-0000-0000-000000000000 \
./target/debug/plugin-dummy

# 用 grpcurl 测试（如果装了）
grpcurl -unix -plaintext /tmp/test.sock momentum.plugin.v1.PluginService/Handshake
```

### 4.3 查 plugin_audit 看历史

```sql
SELECT plugin_id, event, payload, created_at
FROM plugin_audit
ORDER BY created_at DESC
LIMIT 20;
```

### 4.4 重置

```bash
# 卸载 + 重新装
curl -X DELETE http://127.0.0.1:8000/workspaces/{wid}/plugins/{pid} \
  -H "Authorization: Bearer $TOKEN"

# 或直接清表
psql $DATABASE_URL -c "TRUNCATE plugin_installations, plugin_storage, plugin_audit, issue_field_values, issue_field_definitions, plugins, agent_runs, outbox;"
```

---

## 5. 当前限制（v0.1）

- **流式 Agent 输出未实现**（v0.2）
- **View 扩展点是 stub**（v0.2 接入 iframe）
- **Workflow 扩展点是 stub**（v0.2）
- **Integration Hooks（自定义 MCP Server）是 stub**（v0.2）
- **SubscribeEvents 服务端推流未实现**（v0.2 走 NATS）
- **Plugin directory 硬编码**为 `examples/dummy-plugin/`（v0.2 改 config-driven）

不影响核心流程：装、启、停、卸、字段读写、Agent 调用、存储、事件都跑通。

---

## 6. 下一步

| 优先级 | 任务 | 估时 |
|--------|------|------|
| 高 | 写你的真实插件（建议从 MCP Server 集成开始） | 1-2 周 |
| 中 | 接入 NATS（替换内存 channel） | 1 周 |
| 中 | 拆分 Cargo workspace | 1-2 周 |
| 中 | 流式 Agent 输出 | 1 周 |
| 低 | 启动具身智能插件（先达成 Core 软件 PMF） | M6+ |

---

## 7. 文档索引

- [`design.md`](./design.md) — 完整设计（8 大扩展点 / gRPC 协议 / Manifest / 生命周期 / 权限）
- [`handover-2026-06-27.md`](./handover-2026-06-27.md) — P0 实施交接笔记（坑 + 验证命令）
- `proto/plugin.proto` — gRPC 契约
- `plugins/plugin-dummy/plugin.yaml` — 完整 Manifest 示例
- `plugins/plugin-dummy/src/main.rs` — 完整 gRPC Server 示例
- [`src/plugins/`](../src/plugins/) — Core 端 SDK 实现
- [`scripts/smoke_test.sh`](../scripts/smoke_test.sh) — 端到端 smoke test

---

**最后更新**：2026-06-19 · Plugin SDK v0.1
