# 更新日志

本文档记录 Momentum Backend 的重要变更。

格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.0.0/)，
本项目遵循 [语义化版本](https://semver.org/lang/zh-CN/)。

> **2026-07-12 整理**：原 "未发布" 段落已严重滞后于代码（实际功能已落地数月），本次按实际 commit 历史重新整理。版本号反映阶段里程碑，不是 git tag。

---

## [0.3.0] - 2026-07-05 架构审视与可观测性

**主题**：补齐可观测性 + 安全加固 + 架构审视

### 新增

- 🛰 **Prometheus 指标导出**（`9b04ccb`）
  - `momentum_api/src/observability/` 模块
  - 7 个预定义指标：HTTP / DB / WS / Errors
  - `GET /metrics` 端点
- 🔍 **RequestContext.trace_id**（`dd3702f`, `e841434`）
  - 跨服务追踪请求
  - 所有 service 层可读 trace_id
- 📚 **5 篇 ADR**（`fbe25d6`）
  - Axum / Diesel+r2d2 / Repository Pattern / API 版本化 / RBAC 模型
- 📚 **架构审视三件套**（`edbd80b`）
  - ARCHITECTURE_REVIEW（12 维度）
  - ARCHITECTURE_ISSUES（7 问题）
  - REFACTOR_PLAN（分阶段修复）

### 修复（安全与稳定性）

- 🔐 `3f6a7ca` 关键安全修复：工作区隔离 / 访问控制
- 🔐 `b8eb7da` **RBAC 中间件**：工作区角色权限强制执行
- 🔧 `4bca7b6` 连接池耗尽返回 503，不再 `expect()` panic
- 🐳 `5c61924` Dockerfile HEALTHCHECK 修复（安装 curl + 调整时序）
- 🛑 `bbb2904` **graceful shutdown**（SIGTERM / Ctrl+C）
- 🗄 `9802617` Repository trait 抽象 + `spawn_blocking` 包装
- ⚡ `48e10a4` Issue 列表批量加载 teams/workflow，消除 N+1
- 🐛 `ab2e3e7` Redis `KEYS *` 替换为 `SCAN` cursor 迭代

### WebSocket

- ✅ `8a56614` Registry 集成完成 + 测试基础设施
- ✅ `e412400` `create_issue` / `delete_issue` 命令
- ✅ `7d847e0` `switch_workspace` 命令
- ✅ `2231a2a` `get_invitation` 命令
- ✅ `2f14168` cycles 命令
- ✅ `bcfe051` WorkspaceMember 命令
- ✅ `09eeb22` Label 命令
- ✅ `a820ed2` Project 命令
- ✅ `a88f7f1` CommandEnvelope.metadata + MetricsMiddleware（tracing sink）

### 文档（本次一并整理）

- 🗂 14 篇 2025 早期文档归档至 `docs/_archive/2025/`
- 📖 新增模块文档：`docs/observability/`、`docs/plugin/`
- 📖 重写 `docs/websocket/README.md` + 拆分 commands/operations/security
- 📖 重写 `docs/auth/README.md`
- 📖 新增 `docs/superpowers/plans/README.md` 索引
- 📖 新增 `docs/_archive/2025/README.md` 归档总览

---

## [0.2.0] - 2025-10 ~ 2026-06 功能扩展期

**主题**：WebSocket 命令系统 + 业务功能补全

### 新增

- ✨ WebSocket 命令系统：标签 / 团队 / 工作区 / 项目 / 任务 / 项目状态 / 用户
- ✨ 事件订阅（`topics` + `SubscriptionManager`）
- ✨ 速率限制（令牌桶，HTTP + WS）
- ✨ WebSocket 连接监控（实时统计 + 健康检查）
- ✨ 批量操作（标签批量 CRUD）
- ✨ 重试与超时机制
- ✨ 登录/注册响应返回 `current_workspace_url_key`（前端跳转）
- ✨ 用户登出（多设备会话失效 + Redis 缓存清理）
- ✨ Docker / Docker Compose 支持
- ✨ Plugin 系统（v0.1）
  - `momentum_core/src/plugins/`（manifest, permission, extension, registry）
  - `momentum_plugin_host/` gRPC client
  - `plugins/plugin-dummy/` 第一个内部插件
  - `proto/plugin.proto` gRPC 契约
  - DB：新增 plugin/agent_run/issue_field_*/plugin_storage/plugin_audit 表
- ✨ cycles / cycles WS 命令
- ✨ oauth 路由预留

### 改进

- 🔄 登录性能优化（bcrypt 配置 + Redis 缓存）
- 🔄 WebSocket 安全性（JWT 验证 + 速率限制 + 自动清理）
- 🔄 工作区切换优化（缓存 + N+1 减少）

---

## [0.1.0] - 2025-08-08 初始版本

**主题**：单 crate Rust 后端骨架

### 新增

- 🎉 项目初始化（93 次 commit 累计产出）
- 👤 用户认证：JWT + bcrypt + 多设备会话
- 🏢 工作区 + 成员管理 + 邀请
- 📊 项目 + 优先级 + 路线图
- 📝 Issue + 状态流转 + 分配 + 标签
- 👥 团队 + 成员权限
- 🏷 多级标签（workspace / project / issue）
- 🔄 工作流 + 状态
- 💬 评论
- 🔌 WebSocket：连接管理 + 消息广播 + 心跳 + 在线状态
- 🗄 PostgreSQL + Diesel ORM + 迁移
- 💾 Redis 缓存
- 🔐 CORS + 输入验证
- 📊 tracing 结构化日志
- 🧪 单元 / 集成 / 性能测试

---

## 图例

| 标识 | 含义 |
|---|---|
| 🎉 | 里程碑 |
| ✨ | 新增功能 |
| 🔄 | 改进 / 优化 |
| 🐛 | Bug 修复 |
| 🔐 | 安全相关 |
| 🛑 | 可靠性 |
| 📚 | 文档 |
| 🛰 | 可观测性 |
| ⚡ | 性能 |
| 🗄 | 数据库 |
| 🔌 | WebSocket |

---

**维护人**：发布流程自动化前由架构组手工整理
**下一次整理**：每次发布后 / 每月一次