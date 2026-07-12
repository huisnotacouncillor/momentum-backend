# Momentum Backend 文档索引

> 最后更新：2026-07-12（伴随本次文档整理）
> 当前结构：24 篇活跃文档 + 24 篇归档

---

## 🚀 快速入口

| 我是... | 我应该先读... |
|---|---|
| 🆕 新加入的开发者 | 顶层 `README.md` → `docs/architecture/README.md` |
| 🏗 做架构决策 | `docs/adr/README.md` → `docs/architecture/ARCHITECTURE_REVIEW.md` |
| 🔌 接入 WebSocket | `docs/websocket/README.md` → `commands.md` |
| 🛰 加监控/指标 | `docs/observability/README.md` → `metrics.md` |
| 🔌 写插件 | `docs/plugin/README.md` → `design.md` |
| 🔐 改认证逻辑 | `docs/auth/README.md` |
| 🧪 改测试 | `docs/backend-testing-plan.md` |
| 🗂 找历史文档 | `docs/_archive/2025/README.md` |
| 🧭 战略/规划 | `docs/superpowers/plans/README.md` |

---

## 📚 活跃文档（24 篇）

### 架构与决策

| 文档 | 摘要 | 最后更新 |
|---|---|---|
| [architecture/README.md](./architecture/README.md) | 架构文档导航 | 2026-07-05 |
| [architecture/ARCHITECTURE_REVIEW.md](./architecture/ARCHITECTURE_REVIEW.md) | 12 维度审视报告 | 2026-07-05 |
| [architecture/ARCHITECTURE_ISSUES.md](./architecture/ARCHITECTURE_ISSUES.md) | 7 个核心架构问题 | 2026-07-05 |
| [architecture/REFACTOR_PLAN.md](./architecture/REFACTOR_PLAN.md) | 分阶段修复计划 | 2026-07-05 |
| [adr/README.md](./adr/README.md) | ADR 索引 | 2026-07-05 |
| [adr/0001-use-axum.md](./adr/0001-use-axum.md) | ADR: Axum 框架 | 2026-07-05 |
| [adr/0002-diesel-and-r2d2.md](./adr/0002-diesel-and-r2d2.md) | ADR: Diesel + r2d2 | 2026-07-05 |
| [adr/0003-repository-pattern.md](./adr/0003-repository-pattern.md) | ADR: Repository Pattern | 2026-07-05 |
| [adr/0004-api-versioning.md](./adr/0004-api-versioning.md) | ADR: API 版本化 | 2026-07-05 |
| [adr/0005-rbac-model.md](./adr/0005-rbac-model.md) | ADR: RBAC 模型 | 2026-07-05 |

### 业务模块

| 文档 | 摘要 | 最后更新 |
|---|---|---|
| [auth/README.md](./auth/README.md) | JWT 认证、登录、登出、OAuth 预留 | 2026-07-12 |
| [websocket/README.md](./websocket/README.md) | WebSocket 总览 | 2026-07-12 |
| [websocket/commands.md](./websocket/commands.md) | 65+ 命令目录 | 2026-07-12 |
| [websocket/operations.md](./websocket/operations.md) | 压测、监控、故障排查 | 2026-07-12 |
| [websocket/security.md](./websocket/security.md) | HMAC 签名、防重放 | 2026-07-12 |
| [websocket/registry-vs-legacy.md](./websocket/registry-vs-legacy.md) | Registry 双分发问题 | 2026-07-12 |

### 平台能力

| 文档 | 摘要 | 最后更新 |
|---|---|---|
| [observability/README.md](./observability/README.md) | 可观测性导航 | 2026-07-12 |
| [observability/tracing.md](./observability/tracing.md) | 请求追踪 / trace_id | 2026-07-12 |
| [observability/metrics.md](./observability/metrics.md) | Prometheus 指标导出 | 2026-07-12 |
| [observability/known-issues.md](./observability/known-issues.md) | 可观测性已知缺口 | 2026-07-12 |
| [plugin/README.md](./plugin/README.md) | 插件系统实战指南 | 2026-07-12 |
| [plugin/design.md](./plugin/design.md) | 插件 SDK 架构设计 | 2026-06-19 |
| [plugin/handover-2026-06-27.md](./plugin/handover-2026-06-27.md) | 插件 P0 实施交接笔记 | 2026-06-27 |

### 测试与规划

| 文档 | 摘要 | 最后更新 |
|---|---|---|
| [backend-testing-plan.md](./backend-testing-plan.md) | 后端测试计划 | 2026-07-05 |
| [superpowers/plans/README.md](./superpowers/plans/README.md) | 12 篇战略规划索引 | 2026-07-12 |

---

## 🗂 归档（24 篇）

历史版本（2025-08 ~ 2025-10）已归档至 [`_archive/2025/`](./_archive/2025/README.md)，包括：

- 8 篇 WebSocket 早期文档（已合并为新版）
- 3 篇 Auth 早期文档（已合并为新版）
- 5 篇 API 早期设计
- 4 篇 数据库早期文档
- 4 篇散落根目录文档

⚠️ 归档文档保留作为历史参考，**新工作请勿参考**。

---

## 📁 目录结构

```
docs/
├── INDEX.md                          ← 本文件
├── backend-testing-plan.md
├── _archive/
│   ├── README.md                     ← 归档总览
│   └── 2025/                         ← 2025 历史快照
│       ├── api/  (5 篇)
│       ├── auth/ (3 篇)
│       ├── database/ (4 篇)
│       ├── websocket/ (8 篇)
│       └── 散落根目录 4 篇
├── adr/                              ← 架构决策记录
│   ├── README.md
│   ├── template.md
│   └── 0001~0005
├── architecture/                     ← 架构审视与计划
│   ├── README.md
│   ├── ARCHITECTURE_REVIEW.md
│   ├── ARCHITECTURE_ISSUES.md
│   └── REFACTOR_PLAN.md
├── auth/                             ← 认证授权（合并后）
│   └── README.md
├── observability/                    ← 可观测性（新增）
│   ├── README.md
│   ├── tracing.md
│   ├── metrics.md
│   └── known-issues.md
├── plugin/                           ← 插件 SDK（合并后）
│   ├── README.md
│   ├── design.md
│   └── handover-2026-06-27.md
├── websocket/                        ← WebSocket（合并后）
│   ├── README.md
│   ├── commands.md
│   ├── operations.md
│   ├── security.md
│   └── registry-vs-legacy.md
└── superpowers/plans/                ← 战略规划
    ├── README.md
    └── 2026-06-27-*.md (12 篇)
```

---

## 📝 文档维护约定

| 操作 | 谁负责 | 何时 |
|---|---|---|
| 新增模块文档 | 模块 owner | 模块首次合并前 |
| 改 API 行为 | 模块 owner | 同步更新对应 README + INDEX |
| 安全/架构审视 | 架构组 | 每月或大版本前 |
| ADR 起草 | 决策发起人 | 决策前 → 决策中 → Accepted |
| 归档 | 文档组 | 大重构时统一处理 |

---

## 🔗 外部参考

- [Rust 官方文档](https://doc.rust-lang.org/)
- [Axum 框架](https://docs.rs/axum/)
- [Diesel ORM](https://diesel.rs/)
- [Tokio 异步运行时](https://tokio.rs/)
- [tonic gRPC](https://github.com/hyperium/tonic)
- [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)
- [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)

---

**维护人**：架构组
**下次审计**：2026-08-12 或下个大版本前