# 可观测性（Observability）

> 对应代码：`momentum_api/src/observability/`、`momentum_api/src/middleware/{request_tracking,logger,performance_monitoring}.rs`、`momentum_core/src/services/context.rs`
> 引入时间：2026-07-05（commit `9b04ccb` Prometheus + `dd3702f` trace_id）

本目录说明 Momentum Backend 的三类可观测性能力：请求追踪（trace_id）、结构化日志、Prometheus 指标导出。

---

## 文档导航

| 文档 | 内容 |
|---|---|
| **[tracing.md](./tracing.md)** | 请求生命周期、trace_id 生成与传递、`x-request-id` 响应头 |
| **[metrics.md](./metrics.md)** | Prometheus 指标清单、暴露端点、扩展示例 |
| **[known-issues.md](./known-issues.md)** | 当前已知缺口（auth/comments 硬编码 trace_id = "unknown" 等） |

---

## 三件套速览

| 能力 | 实现位置 | 暴露方式 |
|---|---|---|
| **请求追踪** | `middleware/request_tracking.rs` + `services/context.rs` | 响应头 `x-request-id`、日志字段 `request_id`/`trace_id` |
| **结构化日志** | `middleware/logger.rs` + `tracing_subscriber` | stdout JSON（生产）、pretty（开发） |
| **指标导出** | `observability/metrics.rs` | `GET /metrics` Prometheus 文本格式 |

---

## 5 分钟集成示例

### 在路由中加指标

```rust
use crate::observability::metrics::METRICS;

pub async fn create_issue(...) -> Result<...> {
    METRICS.http_requests_total
        .inc(&[axum::http::Method::POST.as_str(), "/issues", "201"])
        .await;
    // ...
}
```

### 在服务层读 trace_id

```rust
pub async fn create(ctx: &RequestContext, ...) -> Result<Issue> {
    tracing::info!(trace_id = %ctx.trace_id, "creating issue");
    // 任何错误日志都能关联到具体请求
}
```

### 拉取指标

```bash
curl http://localhost:8000/metrics
# HELP momentum_http_requests_total ...
# TYPE momentum_http_requests_total counter
momentum_http_requests_total{method="GET",path="/issues",status="200"} 42
```

---

## 相关 ADR / 文档

- `docs/architecture/ARCHITECTURE_REVIEW.md` §3（可观测性维度）
- `docs/architecture/REFACTOR_PLAN.md`（P2/P3 中 trace_id 传递修复项）
- `docs/adr/0004-api-versioning.md`（指标命名遵循的版本化约定）

---

**最后更新**：2026-07-12