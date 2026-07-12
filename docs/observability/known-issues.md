# 可观测性已知缺口（Known Issues）

> 状态：2026-07-13 快照（Issue #1、#2 已修复）
> 来源：架构审视报告 + 代码巡检

## ✅ 已修复（2026-07-13 commit e72244d + 当前）

- **#1 trace_id 在所有路由硬编码 "unknown"** → 改用 `extract_trace_id(&headers)`，新增 72 处接入
- **#3 日志配置不生效** → `init_tracing()` 解析 `Config.log_level` + `LOG_LEVEL` env + `Config.log_format`
- **#6 敏感信息泄漏** → `Config::sanitize_for_logging()` 显式排除 `jwt_secret` / `database_url`，启动日志用 sanitized 版本

---

## 🔴 P0 - 安全 / 数据完整性

无。

---

## 🟠 P1 - 排查能力受损

### 1. 部分路由硬编码 `trace_id = "unknown"`

**位置**：
- `momentum_api/src/routes/auth.rs:86, 116, 146, 175`
- `momentum_api/src/routes/comments.rs:52`

**现象**：

```rust
let ctx = RequestContext {
    user_id: ...,
    workspace_id: ...,
    trace_id: "unknown".to_string(),  // ❌ 应来自 header
    ...
};
```

**后果**：登录、注册、刷新、登出、评论相关日志无法关联到具体请求。

**修复**：

```rust
let trace_id = request
    .headers()
    .get("x-request-id")
    .and_then(|v| v.to_str().ok())
    .unwrap_or("unknown")
    .to_string();

let ctx = RequestContext {
    user_id,
    workspace_id,
    trace_id,
    ...
};
```

**追踪**：见 `docs/architecture/REFACTOR_PLAN.md` P3 修复项。

---

### 2. trace_id 在中间件与 RequestContext 双生成

**位置**：
- `momentum_api/src/middleware/request_tracking.rs:18-25`（生成 request_id 并写入 header）
- `momentum_core/src/services/context.rs:19`（RequestContext::new 再次生成 UUID）

**现象**：理想情况下两者应一致，但目前 RequestContext 不知道中间件生成的 request_id。

**修复方向**：让 middleware 通过 request extensions 注入 trace_id，service 构造 ctx 时从 extensions 读取。

---

## 🟡 P2 - 体验/工具链

### 3. `Config.log_level` / `Config.log_format` 未生效

**位置**：`momentum_api/src/main.rs`（推测仍调用 `tracing_subscriber::fmt::init()` 默认初始化）

**现象**：`Config` 结构里有 `log_level`、`log_format` 字段，从 env 加载，但 `main.rs` 用的是 `fmt::init()`，不读 env。

**修复**：

```rust
use tracing_subscriber::{fmt, EnvFilter};

tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap())
    .json() // 或 .pretty()，根据 config.log_format 决定
    .init();
```

---

### 4. 慢请求阈值硬编码为 1s

**位置**：`momentum_api/src/middleware/request_tracking.rs:99`

**修复**：从 `Config.slow_request_threshold_ms` 读取，默认 1000。

---

### 5. 双 logger 重复记录

**位置**：`request_tracking_middleware` 和 `performance_monitoring_middleware` 都打"Request started/completed"。

**影响**：日志量翻倍，分析噪音增加。

**修复**：合并两个中间件，或明确分层（如 request_tracking 只记 start/end，performance 只记 metrics）。

---

## 🟢 P3 - 长期改进

### 6. 敏感信息泄漏风险

**位置**：`momentum_api/src/main.rs:19`

```rust
tracing::info!("Server starting with config: {:?}", config);
```

`Config` 的 `Debug` 实现会打印 `DATABASE_URL`、`JWT_SECRET`。

**修复**：为 `Config` 手写 `Debug`，只打印非敏感字段，或用 `serde_json` 过滤。

---

### 7. 无 OpenTelemetry / 分布式追踪

**当前**：trace_id 跨 HTTP 单体内部追踪可用，但跨服务（plugin gRPC）尚未透传。

**计划**：momentum_plugin_host 的 gRPC metadata 注入 trace_id。

---

### 8. 无告警规则

**当前**：仅打 `warn!("Slow request detected")`，无主动告警。

**计划**：接 Alertmanager / Grafana Alerting。

---

## 修复优先级建议

| 顺序 | 问题 | 工作量 |
|---|---|---|
| 1 | #1 auth/comments trace_id 硬编码 | 0.5 天 |
| 2 | #2 双 trace_id 统一 | 0.5 天 |
| 3 | #3 log_level 配置生效 | 0.5 天 |
| 4 | #6 敏感信息泄漏 | 0.5 天 |
| 5 | #5 双 logger 合并 | 1 天 |
| 6 | #4 慢请求阈值配置化 | 0.5 天 |
| 7 | #7 OpenTelemetry | 1 周 |
| 8 | #8 告警规则 | 1 周 |