# Prometheus 指标导出（Metrics）

> 实现位置：`momentum_api/src/observability/metrics.rs`、`momentum_api/src/observability/mod.rs`
> 引入：commit `9b04ccb`（2026-07-05）

---

## 1. 设计取舍

**不引入** `prometheus` crate，而是手写轻量级导出器（`metrics.rs` 约 280 行）。原因：
- 避免依赖膨胀
- 当前需求只有 7 个指标，自实现足够
- Prometheus 文本格式非常简单（手写即可）

未来如果指标数量爆炸或需要 histogram_quantile 等高级聚合，再切换到 `prometheus`/`metrics-exporter-prometheus`。

---

## 2. 支持的指标类型

| 类型 | API | 用途 |
|---|---|---|
| Counter | `inc(&[labels])`, `add(&[labels], n)` | 单调递增计数 |
| Gauge | `set(&[labels], val)` | 瞬时值（如连接数） |
| Histogram | `observe(&[labels], val, buckets)` | 延迟分布 |

所有指标都是**标签化的**（`LabeledMetric`），按 labels 组合独立计数。

---

## 3. 预定义指标（`METRICS`）

定义在 `metrics.rs:115-130`：

| 字段 | 类型 | 标签 | 含义 |
|---|---|---|---|
| `http_requests_total` | Counter | `method`, `path`, `status` | HTTP 请求总数 |
| `http_request_duration_ms` | Histogram | `method`, `path` | HTTP 请求耗时（ms） |
| `db_queries_total` | Counter | `query_type`, `table` | DB 查询次数 |
| `db_query_duration_ms` | Histogram | `query_type`, `table` | DB 查询耗时 |
| `ws_connections` | Gauge | `state` | 当前 WS 连接数（active/closing） |
| `ws_messages_total` | Counter | `direction`, `message_type` | WS 消息收发量 |
| `errors_total` | Counter | `layer`, `code` | 错误计数 |

---

## 4. 暴露端点

```rust
// metrics.rs:161
pub async fn prometheus_handler() -> impl axum::response::IntoResponse {
    // 遍历 METRICS，渲染成 Prometheus 文本格式
}
```

挂载在 `/metrics`：

```rust
// routes/health.rs（推测）
.route("/metrics", get(prometheus_handler))
```

### 拉取示例

```bash
$ curl http://localhost:8000/metrics
# HELP momentum_http_requests_total Total HTTP requests
# TYPE momentum_http_requests_total counter
momentum_http_requests_total{method="GET",path="/issues",status="200"} 1428
momentum_http_requests_total{method="POST",path="/issues",status="201"} 87

# HELP momentum_ws_connections Current WebSocket connections
# TYPE momentum_ws_connections gauge
momentum_ws_connections{state="active"} 23
```

---

## 5. 使用示例

### 在路由中计数

```rust
use crate::observability::metrics::METRICS;

pub async fn list_issues(...) -> Result<Json<Vec<Issue>>, AppError> {
    METRICS.http_requests_total
        .inc(&["GET", "/issues", "200"])
        .await;
    Ok(Json(issues))
}
```

### 记录数据库查询耗时

```rust
let start = std::time::Instant::now();
let result = issues::table.load(&mut conn)?;
let duration_ms = start.elapsed().as_millis() as u64;

METRICS.db_queries_total.inc(&["select", "issues"]).await;
METRICS.db_query_duration_ms
    .observe(&["select", "issues"], duration_ms, &[10, 50, 100, 500, 1000])
    .await;
```

### 自定义指标

```rust
use crate::observability::metrics::LabeledMetric;

pub static FOO_REQUESTS: Lazy<LabeledMetric> = Lazy::new(LabeledMetric::new);

// 在 prometheus_handler 里追加导出
// （需要扩展 metrics.rs 的 export 逻辑）
```

---

## 6. Prometheus 抓取配置示例

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'momentum-backend'
    scrape_interval: 15s
    static_configs:
      - targets: ['momentum-backend:8000']
    metrics_path: /metrics
```

### Grafana 常用查询

```promql
# QPS
rate(momentum_http_requests_total[1m])

# P95 延迟
histogram_quantile(0.95, rate(momentum_http_request_duration_ms_bucket[5m]))

# 5xx 错误率
sum(rate(momentum_http_requests_total{status=~"5.."}[5m]))
  / sum(rate(momentum_http_requests_total[5m]))
```

---

## 7. 已知限制

| 限制 | 影响 | 计划 |
|---|---|---|
| Histogram bucket 全局共享 | 不同端点无法用不同 bucket | 短期接受；中期切到 `prometheus` crate |
| 无 `_created` 系列（OpenMetrics） | 部分监控工具要求 | 中期 |
| 不导出单元测试覆盖率 | 与 metrics 无关，需独立工具 | — |
| Label 数量无限制 | 高基数风险（如 user_id 当 label） | 编码规范禁止 |

---

## 相关文档

- [tracing.md](./tracing.md)：日志字段与 request_id
- [known-issues.md](./known-issues.md)：trace_id 在部分路由未连通
- `docs/architecture/REFACTOR_PLAN.md` §可观测性