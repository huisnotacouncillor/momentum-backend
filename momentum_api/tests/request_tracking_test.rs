//! 端到端验证：x-request-id header 通过 request_tracking 中间件传到 route handler。
//!
//! 这是 trace_id 修复 Issue #1 的核心证明：证明"客户端传 header → 中间件透传 → handler 看到正确的值"。

use axum::{
    Router,
    body::Body,
    http::{HeaderName, HeaderValue, Request, StatusCode},
    middleware::{Next, from_fn},
    response::Response,
    routing::post,
};
use std::time::Instant;
use tower::util::ServiceExt;
use tracing::info;
use uuid::Uuid;

/// Mirror of `request_tracking_middleware` — 只为了独立测试，不引入完整 main 依赖
async fn test_tracking<B>(mut request: Request<B>, next: Next<B>) -> Response {
    let start_time = Instant::now();
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    request.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    info!(request_id = %request_id, method = %request.method(), uri = %request.uri(), "Test request started");
    let mut response = next.run(request).await;

    response.headers_mut().insert(
        HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    let _ = start_time; // silence unused while we keep behaviour minimal
    response
}

/// 测试 handler：从 header 读 trace_id，按 extract_trace_id 同款逻辑
/// 通过响应 header `x-echo-trace` 回传，方便断言（避免解析 body）
async fn echo_trace_id_handler<B>(req: Request<B>) -> impl axum::response::IntoResponse {
    let trace_id = momentum_api::middleware::request_tracking::extract_trace_id(req.headers());
    (StatusCode::OK, [("x-echo-trace", trace_id)])
}

fn build_test_app() -> Router {
    Router::new()
        .route("/echo", post(echo_trace_id_handler))
        .layer(from_fn(test_tracking))
}

#[tokio::test]
async fn request_id_header_flows_to_handler() {
    let app = build_test_app();
    let custom_trace = "test-uuid-flow-12345";

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .header("x-request-id", custom_trace)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let echoed = response
        .headers()
        .get("x-echo-trace")
        .expect("handler should expose x-echo-trace")
        .to_str()
        .unwrap();

    assert_eq!(
        echoed, custom_trace,
        "handler should see client-supplied x-request-id"
    );
}

#[tokio::test]
async fn middleware_generates_uuid_when_header_absent() {
    let app = build_test_app();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/echo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // 1) 响应头有 x-request-id（中间件生成）
    let header = response
        .headers()
        .get("x-request-id")
        .expect("response must carry x-request-id")
        .to_str()
        .unwrap()
        .to_string();

    assert!(
        !header.is_empty() && header != "invalid" && header != "unknown",
        "middleware should generate a non-empty trace id, got: {}",
        header
    );

    // 2) handler 看到该 trace_id（不应该是 "unknown"）
    let echoed = response
        .headers()
        .get("x-echo-trace")
        .expect("handler should expose x-echo-trace")
        .to_str()
        .unwrap();
    assert_eq!(
        echoed, header,
        "handler should see the middleware-generated trace id, not 'unknown'"
    );
}
