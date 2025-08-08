use axum::{http::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{Span, info};
use uuid::Uuid;

pub async fn logger<B>(mut req: Request<B>, next: Next<B>) -> Response {
    // 生成 trace_id
    let trace_id = Uuid::new_v4();
    // 将 trace_id 添加到请求扩展，便于后续中间件/处理器使用
    req.extensions_mut().insert(trace_id);
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();

    // 创建 tracing span
    let span = Span::current();
    span.record("trace_id", &tracing::field::display(trace_id));

    // 处理请求
    let response = next.run(req).await;
    let status = response.status().as_u16();
    let elapsed = start.elapsed().as_millis();

    info!(trace_id = %trace_id, method = %method, uri = %uri, status = status, elapsed_ms = elapsed, "Request log");
    response
}
