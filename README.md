# Momentum Backend

<p align="center">
  <strong>面向具身智能团队的研发操作系统后端</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Rust-2024-orange?style=flat-square&logo=rust" alt="Rust 2024">
  <img src="https://img.shields.io/badge/Axum-0.6-blue?style=flat-square" alt="Axum">
  <img src="https://img.shields.io/badge/PostgreSQL-15-blue?style=flat-square&logo=postgresql" alt="PostgreSQL">
  <img src="https://img.shields.io/badge/Redis-7-red?style=flat-square&logo=redis" alt="Redis">
  <img src="https://img.shields.io/badge/WebSocket-Realtime-green?style=flat-square" alt="WebSocket">
  <img src="https://img.shields.io/badge/gRPC-tonic-9cf?style=flat-square" alt="gRPC">
  <img src="https://img.shields.io/badge/Plugin-Ready-purple?style=flat-square" alt="Plugin">
  <img src="https://img.shields.io/badge/Prometheus-Metrics-orange?style=flat-square" alt="Prometheus">
  <img src="https://img.shields.io/badge/Docker-Ready-2496ED?style=flat-square&logo=docker" alt="Docker">
</p>

> **定位**：Momentum 是面向具身智能团队的研发 OS —— 以 Linear 级别的体验，把需求、设计、代码、模型、数据、仿真、真机、部署串成一条可追溯的研发流水线，由 AI Agent 驱动自动化执行。
> **当前阶段**：核心能力（工作区/项目/Issue/团队）+ 插件 SDK + 可观测性已就位。详见 `docs/superpowers/plans/2026-06-27-momentum-product-planning.md`。

---

## ✨ 项目亮点

- 🚀 **三 crate workspace**：`momentum_core`（业务内核）/ `momentum_api`（HTTP + WS）/ `momentum_plugin_host`（gRPC 插件托管）
- 🛰 **可观测性**：Prometheus 指标 + trace_id 跨服务追踪 + 结构化日志
- 🛡 **安全**：JWT + bcrypt + RBAC 中间件 + 工作区隔离（已加固）
- 🔌 **插件化**：8 大扩展点（Field / Agent / Storage / Webhook / Permission ...），gRPC over TCP localhost
- 🪝 **WebSocket**：65+ 命令 + 事件订阅 + Registry/Legacy 双分发
- 🔄 **可靠性**：graceful shutdown / Repository trait 抽象 / SCAN 替代 KEYS / N+1 消除
- 🐳 **生产就绪**：Docker + Docker Compose + 健康检查 + 资源限制

---

## 🚀 核心功能

### 业务能力

| 模块 | 路由文件 | 说明 |
|---|---|---|
| 认证 | `routes/auth.rs` | 注册/登录/Token/登出/OAuth |
| 用户 | `routes/users.rs` | 资料/头像 |
| 工作区 | `routes/workspaces.rs` | 多工作区 + 切换 |
| 成员 | `routes/workspace_members.rs` | 邀请/角色 |
| 项目 | `routes/projects.rs` | CRUD + 状态 |
| 项目状态 | `routes/project_statuses.rs` | 工作流状态 |
| Issue | `routes/issues.rs` | CRUD + 流转 + 自定义字段 |
| 标签 | `routes/labels.rs` | 多级标签 + 批量 |
| 团队 | `routes/teams.rs` | CRUD + 成员 |
| 周期 | `routes/cycles.rs` | 迭代管理 |
| 工作流 | `routes/workflows.rs` | 状态定义 |
| 评论 | `routes/comments.rs` | Issue 评论 |
| 通知 | `routes/notifications.rs` | 系统通知 |
| 自动化 | `routes/automation.rs` | 规则引擎 |
| 插件 | `routes/plugins.rs` | 安装/启用/配置 |
| OAuth | `routes/oauth.rs` | 第三方登录（预留） |
| 健康 | `routes/health.rs` | `/health` |

### WebSocket

`ws://host:8000/ws?token={JWT}` —— 65+ 命令 + 事件订阅，详见 [`docs/websocket/README.md`](./docs/websocket/README.md)。

### 可观测性

| 端点 | 用途 |
|---|---|
| `/health` | 健康检查 |
| `/metrics` | Prometheus 文本格式 |

详见 [`docs/observability/README.md`](./docs/observability/README.md)。

### 插件系统

通过 gRPC + Manifest 把行业垂直能力接入。详见 [`docs/plugin/README.md`](./docs/plugin/README.md)。

---

## 🛠 技术栈

| 层 | 选型 | 版本 |
|---|---|---|
| 语言 | Rust | 2024 edition |
| Web 框架 | Axum | 0.6 |
| 异步运行时 | Tokio | 1.42 |
| WebSocket | tokio-tungstenite | 0.20 |
| ORM | Diesel + r2d2 | 2.0 / 0.8 |
| 数据库 | PostgreSQL | 15+ |
| 缓存 | Redis | 7+ |
| gRPC | tonic + prost | 0.12 / 0.13 |
| 序列化 | serde / serde_json | 1.0 |
| 认证 | jsonwebtoken + bcrypt | 9.3 / 0.10 |
| 验证 | validator | 0.18 (derive) |
| 日志 | tracing | 0.1 |
| 错误处理 | thiserror | 1.0 |
| 构建 | Cargo workspace | 1.78+ |

---

## 📁 项目结构

```
momentum_backend/
├── Cargo.toml                      # workspace root（3 members）
├── Cargo.lock
├── Dockerfile
├── docker-compose.yml
├── docker-compose.test.yml
├── .env.example
├── rust-toolchain.toml
│
├── momentum_core/                  # 业务内核
│   ├── Cargo.toml
│   ├── diesel.toml
│   ├── migrations/                 # Diesel 迁移（~30 个）
│   ├── src/
│   │   ├── lib.rs
│   │   ├── config.rs / error.rs / schema.rs / utils.rs
│   │   ├── context.rs              # RequestContext（含 trace_id）
│   │   ├── services/               # 业务逻辑（auth/issues/projects/...）
│   │   ├── db/
│   │   │   ├── models/             # 数据模型（25+ 张表）
│   │   │   ├── repositories/       # 数据访问层（含 trait 抽象）
│   │   │   └── enums.rs
│   │   ├── plugins/                # 插件 SDK（manifest/permission/extension/registry）
│   │   └── validation/
│   └── tests/
│
├── momentum_api/                   # HTTP + WebSocket 层
│   ├── Cargo.toml
│   ├── src/
│   │   ├── main.rs                 # 入口（含 graceful shutdown）
│   │   ├── lib.rs
│   │   ├── config.rs / error.rs / state.rs
│   │   ├── bin/                    # CLI 工具
│   │   │   ├── websocket_client.rs
│   │   │   ├── websocket_stress_test.rs
│   │   │   └── worker.rs
│   │   ├── cache/                  # Redis 缓存
│   │   ├── middleware/             # auth/cors/request_tracking/logger/...
│   │   ├── observability/          # 🛰 Prometheus 指标
│   │   ├── routes/                 # HTTP 路由（auth/issues/projects/...）
│   │   ├── validation/             # 请求验证
│   │   └── websocket/              # 🪝 WS 协议
│   │       ├── mod.rs              # 状态构造（注意 Registry 未激活）
│   │       ├── manager.rs          # 连接管理
│   │       ├── auth.rs             # JWT 认证
│   │       ├── security.rs         # HMAC 签名
│   │       ├── rate_limiter.rs
│   │       ├── monitoring.rs
│   │       ├── retry_timeout.rs
│   │       ├── protocol/           # 🆕 协议协商
│   │       ├── feature_flags/      # 🆕 FeatureFlag 中间件
│   │       ├── commands/           # 命令系统（Legacy 分发）
│   │       ├── registry/           # 🆕 Registry 分发（尚未激活）
│   │       ├── subscription/       # 🆕 订阅管理器
│   │       ├── events/             # 事件系统
│   │       └── issue_events.rs
│   └── tests/
│
├── momentum_plugin_host/           # gRPC 插件托管
│   ├── Cargo.toml
│   ├── build.rs                    # 生成 proto
│   └── src/
│       ├── lib.rs
│       ├── supervisor.rs           # (plugin_id, workspace_id) → Child
│       ├── process.rs              # spawn / kill
│       └── agent_impl.rs           # gRPC client
│
├── plugins/                        # 插件仓库（独立 workspace member）
│   └── plugin-dummy/               # 内部 dummy 插件
│       ├── Cargo.toml
│       ├── build.rs
│       ├── plugin.yaml             # Manifest
│       └── src/main.rs             # gRPC server
│
├── proto/
│   └── plugin.proto                # 插件 gRPC 契约
│
├── examples/                       # 25+ 个示例 binary
│   ├── simple.rs / test_schema.rs
│   ├── auth/* (login_*, logout_demo, token_auto_renewal_demo)
│   ├── perf/* (bcrypt_performance_test, performance_test, ...)
│   ├── workspace/* (workspace_switching_demo, login_with_workspace_demo)
│   ├── websocket/* (unified_websocket_demo, issues_websocket_demo, websocket_security_demo)
│   └── ...
│
├── docs/                           # 全部文档（24 活跃 + 24 归档）
│   ├── INDEX.md                    # 文档索引
│   ├── adr/                        # 5 篇架构决策
│   ├── architecture/               # 架构审视 3 件套
│   ├── auth/                       # 认证
│   ├── observability/              # 🆕 可观测性
│   ├── plugin/                     # 🆕 插件 SDK（合并后）
│   ├── websocket/                  # WebSocket（合并后）
│   ├── superpowers/plans/          # 12 篇战略规划
│   ├── backend-testing-plan.md
│   └── _archive/2025/              # 24 篇 2025 早期文档归档
│
├── tests/                          # 顶层集成测试
└── scripts/                        # 构建/迁移脚本
```

---

## 🚀 快速开始

### 环境要求

- Rust 2024 edition（rust-toolchain.toml 固定版本）
- PostgreSQL 15+
- Redis 7+
- （可选）Docker / Docker Compose
- （可选）Diesel CLI

### 1. 克隆并配置

```bash
git clone <repository-url>
cd momentum_backend
cp .env.example .env
# 编辑 .env：DATABASE_URL / REDIS_URL / JWT_SECRET 等
```

### 2. 启动依赖

```bash
# 方式 A：用 docker-compose（推荐）
docker-compose up -d postgres redis

# 方式 B：本地装好 PostgreSQL + Redis
```

### 3. 跑迁移

```bash
cd momentum_core
DATABASE_URL=postgres://... diesel migration run --config-file momentum_core/diesel.toml
cd ..
```

### 4. 启动

```bash
# 启动 API
cargo run --bin momentum_api

# （可选）启动 dummy 插件
cargo build --bin plugin-dummy
MOMENTUM_PLUGIN_PORT=19991 ./target/debug/plugin-dummy
```

API 监听 `http://127.0.0.1:8000`，WebSocket 在 `ws://127.0.0.1:8000/ws?token=...`。

### 5. 验证

```bash
# 健康
curl http://localhost:8000/health

# 指标
curl http://localhost:8000/metrics

# 注册
curl -X POST http://localhost:8000/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email":"a@b.com","username":"a","name":"A","password":"pass1234"}'
```

---

## 📡 API 端点速查

完整列表见各模块文档：

| 模块 | 文档 |
|---|---|
| 认证 | [docs/auth/README.md](./docs/auth/README.md) |
| WebSocket | [docs/websocket/commands.md](./docs/websocket/commands.md) |
| 插件 | [docs/plugin/README.md](./docs/plugin/README.md) |

主要 HTTP 端点（部分）：

```
POST   /auth/register
POST   /auth/login
POST   /auth/refresh
POST   /auth/logout
GET    /auth/profile
PUT    /auth/profile

GET    /workspaces
POST   /workspaces
POST   /workspaces/switch

GET    /projects
POST   /projects
GET    /projects/{id}

GET    /issues
POST   /issues
POST   /issues/{id}/transitions

GET    /labels
POST   /labels
POST   /labels/batch    # 批量

POST   /plugins/install
POST   /plugins/{installation_id}/enable
GET    /workspaces/{wid}/plugins

GET    /health
GET    /metrics
GET    /ws/online
GET    /ws/stats

ws://host/ws?token={JWT}
```

---

## 🧪 开发与测试

```bash
# 全量测试
cargo test --workspace

# 单 crate
cargo test --workspace -p momentum_core
cargo test --workspace -p momentum_api

# 压力测试
cargo build --bin websocket_stress_test
./target/debug/websocket_stress_test --test-type all

# 代码质量
cargo fmt
cargo clippy --workspace -- -D warnings

# 跑示例（25+ 个）
cargo run --example login_with_workspace_demo
cargo run --example unified_websocket_demo
cargo run --example logout_demo
cargo run --example token_auto_renewal_demo
```

完整测试计划：[`docs/backend-testing-plan.md`](./docs/backend-testing-plan.md)

---

## 🐳 部署

### Docker Compose（推荐）

```bash
docker-compose up -d
docker-compose logs -f momentum_api
docker-compose down        # 停止
docker-compose down -v     # 停止 + 删除数据卷
```

### 独立 Docker

```bash
docker build -t momentum-backend .
docker run -p 8000:8000 \
  -e DATABASE_URL=postgres://user:pass@host:5432/db \
  -e REDIS_URL=redis://host:6379 \
  -e JWT_SECRET=change-me-in-production \
  momentum-backend
```

### 关键环境变量

| 变量 | 说明 | 必填 |
|---|---|---|
| `DATABASE_URL` | PostgreSQL 连接串 | ✅ |
| `REDIS_URL` | Redis 连接串 | ✅ |
| `JWT_SECRET` | JWT 签名密钥（≥ 32 字符） | ✅ |
| `HOST` / `PORT` | 监听地址 | 默认 127.0.0.1:8000 |
| `WS_MAX_CONNECTIONS` | WS 最大并发 | 默认 10000 |
| `WS_RATE_LIMIT_PER_SECOND` | WS 速率限制 | 默认 10 |
| `DB_POOL_SIZE` | DB 连接池大小 | 默认 20 |
| `MOMENTUM_PLUGIN_PORT` | 插件 gRPC 端口 | 默认 19991 |
| `RUST_LOG` | 日志级别 | 默认 info |

### 健康检查

```dockerfile
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:8000/health || exit 1
```

---

## 📚 文档导航

完整索引：[`docs/INDEX.md`](./docs/INDEX.md)

| 我想... | 看哪篇 |
|---|---|
| 了解架构决策 | [docs/adr/README.md](./docs/adr/README.md) |
| 评估代码质量/技术债 | [docs/architecture/README.md](./docs/architecture/README.md) |
| 接入认证/登录 | [docs/auth/README.md](./docs/auth/README.md) |
| 接入 WebSocket | [docs/websocket/README.md](./docs/websocket/README.md) |
| 加监控/指标 | [docs/observability/README.md](./docs/observability/README.md) |
| 写插件 | [docs/plugin/README.md](./docs/plugin/README.md) |
| 看测试计划 | [docs/backend-testing-plan.md](./docs/backend-testing-plan.md) |
| 找历史/规划 | [docs/superpowers/plans/README.md](./docs/superpowers/plans/README.md) |
| 查变更 | [CHANGELOG.md](./CHANGELOG.md) |
| 贡献 | [CONTRIBUTING.md](./CONTRIBUTING.md) |

---

## ⚠️ 已知技术债

完整列表：[`docs/architecture/ARCHITECTURE_ISSUES.md`](./docs/architecture/ARCHITECTURE_ISSUES.md)

| 优先级 | 问题 | 状态 |
|---|---|---|
| 🔴 P0 | 工作区隔离失效 | ✅ 已修复（`3f6a7ca`） |
| 🔴 P0 | RBAC 缺失 | ✅ 已修复（`b8eb7da`） |
| 🟠 P1 | 搜索绕过 GIN 索引 | 计划 P2 |
| 🟠 P1 | WebSocket Registry 未激活（双分发） | 计划 P2 |
| 🟡 P2 | Refresh Token 无旋转 | 计划 P3 |
| 🟡 P2 | trace_id 在部分路由未连通 | 计划 P3 |
| 🟡 P2 | 无熔断器 | 计划 P3 |

---

## 🤝 贡献

详见 [CONTRIBUTING.md](./CONTRIBUTING.md)

提交规范：Conventional Commits（中文模板见 `.claude/skills/chinese-commit-conventions/`）

```bash
git checkout -b feature/your-feature
# TDD：先写测试 → 再实现 → 再重构
git commit -m "feat(auth): add OAuth Google login"
git push origin feature/your-feature
# 开 PR
```

---

## 📄 许可证

待定（见 [`PROJECT_OVERVIEW.md`](./PROJECT_OVERVIEW.md)）

---

## 📧 联系方式

通过 Issue 提交问题或建议。

---

<p align="center">
  Made with ❤️ using Rust · Momentum 是面向具身智能的研发 OS
</p>