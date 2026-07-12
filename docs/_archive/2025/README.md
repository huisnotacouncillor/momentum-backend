# 2025 历史文档归档

> ⚠️ **这些文档为 2025-08 ~ 2025-10 期间编写的实现快照**，覆盖**早期单 crate 结构**和**未引入插件/可观测性的版本**。
> 当前代码已演进（workspace 三 crate、可观测性、插件系统、Registry 重构），其中部分路径、模块名、API 与现状不符。
> **新工作请优先参考根目录 `docs/` 下的最新文档。**

---

## 📚 归档清单（24 篇）

### API 早期设计（5 篇）

| 文档 | 原位置 | 处置 |
|---|---|---|
| [api/API_RESPONSE_DESIGN.md](./api/API_RESPONSE_DESIGN.md) | `docs/api/` | 内容仍部分适用，但需对照 `momentum_core/src/db/models/api.rs` 确认 |
| [api/PROJECT_API_IMPLEMENTATION.md](./api/PROJECT_API_IMPLEMENTATION.md) | `docs/api/` | 项目路由已加入 cycles/automation 等模块，需更新 |
| [api/WORKSPACE_SWITCHING_API.md](./api/WORKSPACE_SWITCHING_API.md) | `docs/api/` | 登录响应新增 `current_workspace_url_key` 字段未覆盖 |
| [api/PROFILE_API_UPDATED.md](./api/PROFILE_API_UPDATED.md) | `docs/api/` | profile 与 assets 两条文档已合并 |
| [api/ISSUE_TRANSITIONS_API.md](./api/ISSUE_TRANSITIONS_API.md) | `docs/api/` | 路由位置可能已迁移，需对照 `momentum_api/src/routes/issues.rs` |

### 数据库（4 篇）

| 文档 | 原位置 | 处置 |
|---|---|---|
| [database/MOMENTUM_SCHEMA_IMPLEMENTATION.md](./database/MOMENTUM_SCHEMA_IMPLEMENTATION.md) | `docs/database/` | 后续又新增 `plugin_installations` / `plugin_storage` / `plugin_audit` / `issue_field_definition` / `issue_field_value` / `agent_run` 等表 |
| [database/MODELS_REFACTORING_SUMMARY.md](./database/MODELS_REFACTORING_SUMMARY.md) | `docs/database/` | 历史重构记录 |
| [database/RELATIONSHIP_MODEL_SUMMARY.md](./database/RELATIONSHIP_MODEL_SUMMARY.md) | `docs/database/` | 表关系说明，建议改读 `momentum_core/src/schema.rs` |
| [database/SWITCH_WORKSPACE_OPTIMIZATION.md](./database/SWITCH_WORKSPACE_OPTIMIZATION.md) | `docs/database/` | 工作区切换优化实验记录 |

### 认证（3 篇，已合并）

| 文档 | 原位置 | 处置 |
|---|---|---|
| [auth/README.md](./auth/README.md) | `docs/auth/` | 已合并为 `docs/auth/README.md`（新版） |
| [auth/LOGOUT_API.md](./auth/LOGOUT_API.md) | `docs/auth/` | 内容已纳入新版 |
| [auth/LOGOUT_IMPLEMENTATION_SUMMARY.md](./auth/LOGOUT_IMPLEMENTATION_SUMMARY.md) | `docs/auth/` | 内容已纳入新版 |

### WebSocket（8 篇，已合并）

| 文档 | 原位置 | 处置 |
|---|---|---|
| [websocket/README.md](./websocket/README.md) | `docs/websocket/` | 已合并为 `docs/websocket/README.md` + `commands.md` + `security.md` + `operations.md` |
| [websocket/IMPLEMENTATION_SUMMARY.md](./websocket/IMPLEMENTATION_SUMMARY.md) | 同上 | 同上 |
| [websocket/CHECKLIST.md](./websocket/CHECKLIST.md) | 同上 | 同上 |
| [websocket/SECURITY.md](./websocket/SECURITY.md) | 同上 | 同上 |
| [websocket/SECURITY_IMPLEMENTATION_SUMMARY.md](./websocket/SECURITY_IMPLEMENTATION_SUMMARY.md) | 同上 | 同上 |
| [websocket/INITIAL_DATA_FEATURE.md](./websocket/INITIAL_DATA_FEATURE.md) | 同上 | 同上 |
| [websocket/ISSUES_WEBSOCKET_IMPLEMENTATION.md](./websocket/ISSUES_WEBSOCKET_IMPLEMENTATION.md) | 同上 | 同上 |
| [websocket/workspace-commands.md](./websocket/workspace-commands.md) | `docs/websocket-workspace-commands.md` | 内容已纳入 `docs/websocket/commands.md` |

### 散落文档（4 篇）

| 文档 | 原位置 | 处置 |
|---|---|---|
| [ASSETS_URL_IMPLEMENTATION.md](./ASSETS_URL_IMPLEMENTATION.md) | `docs/` | 资源 URL 处理 |
| [PROFILE_API_ASSETS_UPDATE.md](./PROFILE_API_ASSETS_UPDATE.md) | `docs/` | profile 与 assets 合并更新 |
| [comment_feature.md](./comment_feature.md) | `docs/` | 评论功能说明 |
| [LOGIN_PERFORMANCE_OPTIMIZATION.md](./LOGIN_PERFORMANCE_OPTIMIZATION.md) | `docs/` | 登录性能实验记录 |

---

## 🔍 当前适用文档地图

| 我想... | 看哪篇 |
|---|---|
| 了解项目总体 | 顶层 `README.md` |
| 找所有文档 | `docs/INDEX.md` |
| 认证/登录/登出 | `docs/auth/README.md` |
| WebSocket 命令 | `docs/websocket/commands.md` |
| WS 安全 | `docs/websocket/security.md` |
| WS 运维/压测 | `docs/websocket/operations.md` |
| 可观测性/指标 | `docs/observability/README.md` |
| 插件系统 | `docs/plugin/README.md` |
| 架构决策 | `docs/adr/README.md` |
| 架构审视 | `docs/architecture/README.md` |
| 战略规划 | `docs/superpowers/plans/README.md` |

---

## 📝 何时回看归档

仅当以下情况才需要翻归档：
1. 排查"以前为什么这样写"
2. 写 changelog/migration guide 时追溯历史
3. 评估"之前那个方案是不是更优"做对比
4. 写"项目演进史"文章

---

**归档日期**：2026-07-12
**维护人**：文档组
**保留策略**：永久保留（除非文件损坏）