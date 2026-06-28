# Momentum 产品规划文档（修订版）

> **面向 AI 代理的工作者：** 本文档是产品规划总纲。**基于现有代码库存盘点结果修订**，标注了哪些已实现、哪些需要新增。
>
> **最新盘点日期**：2026-06-27

---

## 一、产品愿景与定位

### 1.1 一句话定位

> **Momentum 是面向具身智能团队的"研发操作系统"：以 Linear 级别的体验，把需求、设计、代码、模型、数据、仿真，真机、部署串成一条可追溯的研发流水线，并由 AI Agent 驱动自动化执行。**

### 1.2 现有代码盘点结果

#### momentum_core ✅ 已实现（主要）
- **37 张表**，24 个模型，19 个 repository，15 个 service
- 核心业务：Auth / Workspace / Teams / Projects / Cycles / Labels / Comments / Workflows **全已实现**
- **Plugin 系统**：Field / Agent / Storage / Event 4 个扩展点**已实现**
  - `manifest.rs`：Manifest 解析验证（14 tests）
  - `extension/field.rs`：自定义字段
  - `extension/agent.rs`：Agent 调用追踪（`agent_runs` 表 + repo）
  - `extension/storage.rs`：插件存储
  - `extension/event.rs`：事件发布
  - `registry/`：内存插件注册表
  - `permission.rs`：权限校验

#### momentum_api ✅ 已实现
- **Axum + REST/JSON** API（约 60 个端点）
- **WebSocket** 完整基础设施：handler / manager / commands / events / rate_limiter / retry_timeout / monitoring / security
- **Plugin HTTP 路由**（刚实现的 7 个端点）
- 中间件：JWT 鉴权 / Logger / RequestTracking / Performance
- Redis 缓存层

#### momentum_plugin_host ✅ 已实现
- `Supervisor`：插件进程管理（TCP-based v0.1）
- `process.rs`：spawn/terminate/ping
- `agent_impl.rs`：gRPC lazy 连接 + `invoke_agent`

#### 前端 ❌ 不存在
- 需要从零搭建 Next.js + React + Zustand

#### 以下模块 ❌ 不存在（需新增）
- `devices` / `fleets` 表
- `artifacts` 表（跨学科制品关联）
- `model_versions` / `sim_scenes` / `sim_runs` 表
- MCP Gateway（GitHub / Figma / 设备）
- Agent Orchestrator（Spec / Code / Test / Diagnose Agent）
- 前端（Next.js）
- OTA 部署服务

---

## 二、产品架构（四层模型）

```
L1 · 工作流层 (Workflow Layer)          ← ✅ 现有 Issue/Project/Cycle/Roadmap
L2 · 制品层 (Artifact Layer)          ← ❌ 新增（artifacts 表）
L3 · 智能层 (Intelligence Layer)      ← ❌ 新增（MCP + Agent）
L4 · 物理层 (Physical Layer)          ← ❌ 新增（devices 表 + OTA）
```

---

## 三、技术架构（基于现有代码修订）

### 3.1 总体架构（修订版）

```
┌──────────────────────────────────────────────────────────────┐
│  Frontend (Next.js + React + Zustand + TailwindCSS)          │
│  ← 前端需要从零搭建                                           │
└─────────────────────┬────────────────────────────────────────┘
                      │ HTTP/JSON + WebSocket
┌─────────────────────▼────────────────────────────────────────┐
│  momentum_api (Axum)                                         │
│  · REST API (60 endpoints)         ✅ 已存在                  │
│  · WebSocket (handler/manager)    ✅ 已存在                  │
│  · Plugin HTTP routes (7 endpoints) ✅ 刚实现                │
│  · middleware: auth/cache/tracking                          │
└─────────────────────┬────────────────────────────────────────┘
                      │ diesel ORM
┌─────────────────────▼────────────────────────────────────────┐
│  momentum_core (Pure Rust, no HTTP)                         │
│  · 37 tables / 24 models / 19 repos / 15 services ✅        │
│  · Plugin extension system (Field/Agent/Storage/Event) ✅     │
│  · ❌ 新增: devices, artifacts, model_versions, sim_scenes   │
└─────────────────────┬────────────────────────────────────────┘
                      │
┌─────────────────────▼────────────────────────────────────────┐
│  momentum_plugin_host (gRPC client)                          │
│  · Supervisor (plugin process) ✅ 已存在（TCP v0.1）          │
│  · Agent gRPC client (stub)     ✅ 已存在                    │
│  · ❌ 新增: Agent Orchestrator / MCP Gateway                 │
└──────────────────────────────────────────────────────────────┘
```

### 3.2 技术选型（修订版）

| 维度 | 原计划 | 修订后（基于现有代码）|
|------|--------|---------------------|
| 后端框架 | NestJS + GraphQL | **Axum + REST/JSON** ✅ 已有 |
| API 协议 | GraphQL | **REST/JSON + WebSocket** ✅ 已有 |
| 数据库 | PostgreSQL 16 | **PostgreSQL + Diesel ORM** ✅ 已有 |
| 实时同步 | 独立 Sync Service (Node) | **已有 WS 在 momentum_api（Rust）** ✅ 需增强 |
| 前端 | React + Next.js | ❌ 需从零搭建 |
| 状态管理 | Zustand + Replicache | ❌ Replicache 暂不需要（先用 Zustand） |
| AI Agent | TypeScript + LangGraph | **Rust + 现有 gRPC client**（架构需重新设计）|
| MCP | TypeScript SDK | ❌ 需新增 |
| Plugin 进程 | 独立 TypeScript 服务 | **已有 Rust Supervisor** ✅ |
| 缓存 | Redis Cluster | **已有 Redis Client** ✅ |
| CI/CD | GitHub Actions 集成 | ❌ 需新增 |

---

## 四、功能模块（修订版状态）

| 模块 | 状态 | 备注 |
|------|------|------|
| M1. 需求 & 工作流 | ✅ 已有 | Issue/Project/Cycle/Label/Workflow 全了 |
| M2. 设计协作（Figma）| ❌ 需新增 | 需 Figma MCP Server |
| M3. AI Agent 平台 | ⚠️ 部分有 | gRPC client stub 已有，Orchestrator/MCP 需新增 |
| M4. 代码 & 制品关联 | ⚠️ 部分有 | GitHub 集成已有（plugin_host），artifacts 表需新增 |
| M5. 模型 & 数据管理 | ❌ 需新增 | model_versions 等表不存在 |
| M6. 仿真 & 真机 | ❌ 需新增 | devices/firmware 表不存在 |
| M7. CI/CD & 部署 | ❌ 需新增 | 暂无 GitHub Actions 集成 |
| M8. 观测 & 现场运维 | ❌ 需新增 | 设备遥测/OTA 不存在 |

---

## 五、现有 Plugin 系统详解（已实现）

### 5.1 扩展点现状

| 扩展点 | 状态 | 代码位置 |
|--------|------|---------|
| **Field** (issue.* 自定义字段) | ✅ 完整 | `extension/field.rs` + `issue_field_definitions` + `issue_field_values` 表 |
| **Agent** (AI Agent 调用) | ✅ 表和 repo 完整 | `extension/agent.rs` + `agent_runs` 表 |
| **Storage** (插件 KV) | ✅ 完整 | `extension/storage.rs` + `plugin_storage` 表 |
| **Event** (事件发布) | ✅ 完整 | `extension/event.rs` + `outbox` + `plugin_audit` 表 |
| View | ❌ stub | 待实现 |
| Workflow | ❌ stub | 文档有设计 |
| Integration | ❌ stub | 文档有设计 |
| Webhook | ✅ 有结构 | `WebhookDef` 在 manifest 里 |

### 5.2 Plugin 生命周期

```
Install → Disable → Enable ↔ Running ↔ Error
            ↓
         Supervisor.start() → spawn plugin binary → wait TCP port
```

- **v0.1**：TCP localhost（已完成）
- **v0.2**：切换 Unix Domain Socket（待做）

### 5.3 现有 Plugin HTTP 路由（刚实现）

| 方法 | 路径 | 功能 |
|------|------|------|
| GET | `/plugins` | 列出可用插件 |
| POST | `/plugins/install` | 安装插件 |
| POST | `/plugins/:inst_id/enable` | 启用插件 |
| POST | `/plugins/:inst_id/disable` | 禁用插件 |
| GET | `/workspaces/:wid/plugins` | 列出已装插件 |
| DELETE | `/workspaces/:wid/plugins/:pid` | 卸载插件 |
| GET | `/workspaces/:wid/fields` | 列出字段定义 |

---

## 六、新增数据模型（需实现）

### 6.1 artifacts（跨学科制品关联）

一个 Issue 可关联：代码 PR + Figma 设计稿 + 模型权重 + 数据集 + CAD + 固件 + 仿真报告 + 测试报告 + 部署记录

```sql
artifacts (
  id, workspace_id,
  type ENUM('code','pr','design','model','dataset','cad','firmware','sim_report','test_report','deploy'),
  ref_id,        -- 外部引用（GitHub PR# / Figma file key / S3 key）
  url,
  metadata JSONB, -- type-specific info
  linked_issue_id, -- 关联 Issue
  version,
  device_id,      -- 部署类制品关联设备
  created_by, created_at
)
```

### 6.2 devices / fleets（设备管理）

```sql
devices (
  id, workspace_id, fleet_id,
  name, serial_number, type,
  hardware_version, firmware_version, software_version,
  model_artifact_id,   -- 当前模型版本
  status ('online'|'offline'|'error'|'maintenance'),
  telemetry JSONB,      -- 最新遥测
  last_seen_at
)

fleets (
  id, workspace_id, name, description
)

firmware_versions (
  id, device_type, version, artifact_id,
  changelog, is_stable
)

device_firmware_history (
  device_id, from_version, to_version, deployed_at, deployed_by
)
```

### 6.3 model_versions / sim_scenes / sim_runs

```sql
model_versions (
  id, workspace_id, name, version, description,
  artifact_id,         -- 模型文件 artifact
  trained_on_dataset_id,-- 数据集 artifact
  training_code_artifact_id,
  metrics JSONB,        -- accuracy, latency...
  parent_model_version_id,
  is_production, production_deployment_id,
  created_by, created_at
)

sim_scenes (
  id, workspace_id, name, environment,
  config JSONB, artifact_id, created_by
)

sim_runs (
  id, issue_id, scene_id,
  status, artifact_id,  -- sim_report artifact
  metrics JSONB,
  started_at, completed_at
)
```

---

## 七、路线图（四阶段修订版）

### Phase 1（M0–M2）：Issue 系统完善

| 任务 | 依赖 | 工期 |
|------|------|------|
| **A1: 修复 `update_fields` 多字段 bug** | 无 | M0-W1 |
| **A2: 统一 API 返回类型（IssueResponse）** | A1 | M0-W1–W2 |
| **A3: WebSocket issue handlers 实现** | A2 | M0-W2 |
| **B1: 分页支持** | A3 | M0-W3 |
| **B2: 团队 `issue_number`（ENG-123）** | 无 | M0-W3 |
| **B3: DB 层过滤（替代内存过滤）** | B1 | M0-W3 |

**参考**：`plans/2026-06-27-issue-system.md`

### Phase 2（M3–M4）：后端增强 + Agent 核心

| 任务 | 依赖 | 工期 |
|------|------|------|
| P0-D1: 新增数据表（artifacts + devices + model_versions） | 无 | M0-W3–W4 |
| P0-A1: Rust MCP Gateway（GitHub + Figma） | 现有 agent_impl | M0-W4–M1 |
| P0-A2: Agent Orchestrator（Spec + Code Agent） | A1 | M1 |

### Phase 3（M4–M6）：制品关联 + 部署

| 任务 | 依赖 |
|------|------|
| P1-A1: CI/CD 集成（GitHub Actions Webhook） | 现有 WS |
| P1-A2: OTA 灰度部署服务 | P0-D1 |
| P1-M1: W&B / MLflow 集成（训练任务追踪） | P0-A1 |
| P1-M2: 模型注册表 API + 前端 | P0-D1 |

### Phase 4（M7–M9）：物理世界

| 任务 | 依赖 |
|------|------|
| P2-S1: 仿真集成（Isaac Sim / Gazebo 适配器） | P1-M1 |
| P2-S2: 设备管理 + 遥测 API | P0-D1 |
| P2-S3: Diagnose Agent（根因分析） | P2-S1 + P2-S2 |

### Phase 5（M10–M12）：生态

| 任务 | 依赖 |
|------|------|
| P3-M1: MCP 开放平台（第三方 Server） | P0-A1 |
| P3-M2: Desktop 客户端（Electron） | 全部后端 |
| P3-M3: 行业模板 + 私有化部署 | 全部 |

---

## 八、模块拆分清单

| # | 模块名称 | 计划文档 | 状态 |
|---|---------|---------|------|
| **Issue 系统（P0-M1）** | | | |
| A1 | 修复 `update_fields` 多字段 bug | `plans/2026-06-27-issue-system.md` | ❌ 待执行 |
| A2 | 统一 API 返回 IssueResponse | `plans/2026-06-27-issue-system.md` | ❌ 待执行 |
| A3 | WebSocket issue handlers | `plans/2026-06-27-issue-system.md` | ❌ 待执行 |
| B1 | 分页支持 | `plans/2026-06-27-issue-system.md` | ❌ 待执行 |
| B2 | 团队 `issue_number`（ENG-123）| `plans/2026-06-27-issue-system.md` | ❌ 待执行 |
| B3 | DB 层过滤（替代内存过滤）| `plans/2026-06-27-issue-system.md` | ❌ 待执行 |
| C1 | Bulk update/delete | `plans/2026-06-27-issue-system.md` | ❌ 待建 |
| C2 | sort 控制 | `plans/2026-06-27-issue-system.md` | ❌ 待建 |
| C3 | `IssueFieldDefinitionRepo` | `plans/2026-06-27-issue-system.md` | ❌ 待建 |
| C4 | Issue 关系（blocks/blocked_by）| `plans/2026-27-issue-system.md` | ❌ 待建 |
| **新增数据表（P0-M2）** | | | |
| P0-D1 | 新增数据表（artifacts + devices + model_versions 等） | `plans/2026-06-27-new-tables.md` | ✅ 已创建 |
| **Agent 平台** | | | |
| P0-A1 | Rust MCP Gateway（GitHub + Figma） | `plans/2026-06-27-mcp-gateway.md` | ❌ 待建 |
| P0-A2 | Agent Orchestrator（Spec + Code Agent） | `plans/2026-06-27-agent-orchestrator.md` | ❌ 待建 |
| **CI/CD + 部署** | | | |
| P1-A1 | CI/CD GitHub Actions 集成 | `plans/2026-06-27-cicd-deploy.md` | ❌ 待建 |
| P1-A2 | OTA 灰度部署服务 | `plans/2026-06-27-ota-service.md` | ❌ 待建 |
| **MLOps** | | | |
| P1-M1 | W&B / MLflow 集成 | `plans/2026-06-27-mlops.md` | ❌ 待建 |
| **物理世界** | | | |
| P2-S1 | 仿真集成（Isaac Sim / Gazebo） | `plans/2026-06-27-simulation.md` | ❌ 待建 |
| P2-S2 | 设备管理 + 遥测 | `plans/2026-06-27-device-ota.md` | ❌ 待建 |
| P2-S3 | Diagnose Agent | `plans/2026-06-27-diagnose-agent.md` | ❌ 待建 |
| **生态** | | | |
| P3-M1 | MCP 开放平台 | `plans/2026-06-27-mcp-platform.md` | ❌ 待建 |
| P3-M2 | Desktop 客户端 | `plans/2026-27-desktop-client.md` | ❌ 待建 |

**参考文档**：
- `plans/2026-06-27-existing-backend.md` — 现有后端架构说明（已有代码详情）
- `plans/2026-06-27-momentum-product-planning.md` — 本文档（总纲）

---

## 九、关键约束（基于现有代码）

1. **后端必须是 Rust/Axum**：所有新增服务用 Rust 实现，不能引入 Node.js/NestJS
2. **API 协议是 REST/JSON + WebSocket**：不能用 GraphQL（除非新增独立 GraphQL 层）
3. **Plugin 用 gRPC（TCP v0.1）**：Agent 通过 `momentum_plugin_host::agent_impl` 调用
4. **momentum_core 无 HTTP 依赖**：所有新增逻辑在 `momentum_core` 里必须是 pure Rust
5. **momentum_api 持有 Supervisor**：AppState 已含 `plugin_host: Arc<Supervisor>`

---

## 十、立即可执行的下一步

1. **删除错误计划**：删除 `backend-scaffold.md`、`sync-layer.md`
2. **新建正确计划**：
   - `frontend-scaffold.md`（Next.js 从零）
   - `new-tables.md`（artifacts + devices + model_versions）
   - `mcp-gateway.md`（Rust MCP，不是 TypeScript）
3. **现有代码文档化**：为 `momentum_api` 的 WebSocket 基础设施和 `momentum_plugin_host` 的 gRPC client 写使用文档

---

> **文档版本**：2026-06-27（修订版，基于代码库存盘点）
> **下一步**：从 P0-F1（前端脚手架）或 P0-D1（新增数据表）开始
