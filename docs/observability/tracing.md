# 请求追踪（Tracing）

> 实现位置：`momentum_api/src/middleware/request_tracking.rs`、`momentum_api/src/middleware/logger.rs`、`momentum_core/src/services/context.rs`

---

## 1. 设计目标

- 每个 HTTP 请求有唯一 ID，前后端共享
- 任何错误日志可关联到具体请求
- 慢请求（> 1s）自动告警

---

## 2. 生命周期

```
客户端
  │  (可选) 携带 x-request-id
  ▼
HTTP 入口 → request_tracking_middleware
  │  - 读 header 或生成 UUID v4
  │  - 注入响应头 x-request-id
  │  - info!("Request started", request_id, method, uri)
  ▼
Axum 路由 → service::xxx(&RequestContext { trace_id, ... })
  │  - 所有 tracing 宏可读 ctx.trace_id
  ▼
响应回写
  │  - 响应头: x-request-id
  │  - info!("Request completed", request_id, status, duration_ms)
  │  - duration_ms > 1000 → warn!("Slow request detected")
  ▼
客户端拿到响应头中的 x-request-id
```

---

## 3. 中间件

### `request_tracking_middleware`

位于 `momentum_api/src/middleware/request_tracking.rs:15-110`：

```rust
pub async fn request_tracking_middleware<B>(
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
    let start_time = Instant::now();
    let request_id = get_or_generate_request_id(request.headers()); // UUID v4

    // 注入响应头
    request.headers_mut().insert(
        HeaderName::from_static(REQUEST_ID_HEADER),
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    info!(request_id = %request_id, method = %method, uri = %uri, "Request started");
    let mut response = next.run(request).await;
    let duration_ms = start_time.elapsed().as_millis();

    response.headers_mut().insert(/* x-request-id */);

    if duration_ms > 1000 {
        warn!(request_id = %request_id, duration_ms = %duration_ms, "Slow request detected");
    }
    response
}
```

**响应头常量**：`REQUEST_ID_HEADER = "x-request-id"`

### `get_or_generate_request_id`

```rust
fn get_or_generate_request_id(headers: &HeaderMap) -> String {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}
```

如果客户端已经传了 `x-request-id`（如来自网关/上游追踪），就复用。

---

## 4. RequestContext 中的 trace_id

`momentum_core/src/services/context.rs:9-19`：

```rust
pub struct RequestContext {
    /// P3.3 修复：trace_id 用于跨服务追踪请求
    pub trace_id: String,
    pub user_id: Uuid,
    pub workspace_id: Uuid,
    // ...
}

impl RequestContext {
    pub fn new(user_id: Uuid, workspace_id: Uuid) -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(), // 自动生成
            // ...
        }
    }
}
```

**当前状态**：trace_id 由中间件写入 header，但**部分路由**（auth、comments）构造 `RequestContext` 时硬编码 `"unknown"`，详见 `known-issues.md`。

---

## 5. 日志字段约定

| 字段 | 来源 | 示例 |
|---|---|---|
| `request_id` | `request_tracking_middleware` | `request_id = "550e8400-e29b-41d4-a716-446655440000"` |
| `trace_id` | `RequestContext`（理想情况与 request_id 同值） | `trace_id = "550e8400-..."` |
| `method` | HTTP method | `method = "POST"` |
| `uri` | 完整 URI | `uri = "/issues"` |
| `status` | 响应状态码 | `status = "201"` |
| `duration_ms` | 处理耗时 | `duration_ms = 42` |
| `user_agent` | 请求头 | `user_agent = "Mozilla/5.0 ..."` |

---

## 6. 慢请求告警

`request_tracking.rs:99-107`：

```rust
if duration_ms > 1000 {
    warn!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        duration_ms = %duration_ms,
        "Slow request detected"
    );
}
```

阈值（1s）目前是硬编码。可改造为 `Config` 字段：

```rust
// TODO: 从 env 读取
const SLOW_REQUEST_THRESHOLD_MS: u128 = 1000;
```

---

## 7. 排查示例

客户端报告"创建 Issue 偶尔 5xx"，拿到响应头 `x-request-id: abc-123`：

```bash
# 日志中搜
grep "abc-123" backend.log

# 应该看到：
# Request started   method=POST uri=/issues request_id=abc-123
# Request completed status=500 duration_ms=2340 request_id=abc-123
# Slow request detected ... request_id=abc-123
# （然后是 service 层带 trace_id 的错误日志）
```

---

## 已知缺口

见 [known-issues.md](./known-issues.md)。