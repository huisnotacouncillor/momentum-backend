use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

/// 请求ID头部名称
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// 请求追踪中间件
/// 为每个请求生成唯一ID，记录请求信息和响应时间
pub async fn request_tracking_middleware<B>(mut request: Request<B>, next: Next<B>) -> Response {
    let start_time = Instant::now();

    // 生成或获取请求ID
    let request_id = get_or_generate_request_id(request.headers());

    // 将请求ID添加到请求头中，供后续处理器使用
    request.headers_mut().insert(
        HeaderName::from_static(REQUEST_ID_HEADER),
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    // 记录请求开始信息
    let method = request.method().clone();
    let uri = request.uri().clone();
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");

    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        user_agent = %user_agent,
        "Request started"
    );

    // 处理请求
    let mut response = next.run(request).await;

    // 计算处理时间
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis();

    // 添加请求ID到响应头
    response.headers_mut().insert(
        HeaderName::from_static(REQUEST_ID_HEADER),
        HeaderValue::from_str(&request_id).unwrap_or_else(|_| HeaderValue::from_static("invalid")),
    );

    // 记录请求完成信息
    let status = response.status();

    if status.is_success() {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration_ms,
            "Request completed successfully"
        );
    } else if status.is_client_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration_ms,
            "Request completed with client error"
        );
    } else if status.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration_ms,
            "Request completed with server error"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            status = %status,
            duration_ms = %duration_ms,
            "Request completed"
        );
    }

    // 性能监控：记录慢请求
    if duration_ms > 1000 {
        warn!(
            request_id = %request_id,
            method = %method,
            uri = %uri,
            duration_ms = %duration_ms,
            "Slow request detected"
        );
    }

    response
}

/// 获取或生成请求ID
fn get_or_generate_request_id(headers: &HeaderMap) -> String {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

/// 性能监控中间件（Issue #8 修复后为纯透传）
///
/// 历史：之前该函数也记录 perf metrics 与分级日志，与
/// `request_tracking_middleware` 的完成日志重复，导致每个请求产生 2 条完成日志。
///
/// 修复后：`request_tracking_middleware` 已经记录 status + duration_ms
/// 在结构化字段中，无需重复打日志。本函数保留只是为了 Router 装配兼容，
/// 是纯透传。
pub async fn performance_monitoring_middleware<B>(request: Request<B>, next: Next<B>) -> Response {
    next.run(request).await
}

/// 从请求头中提取请求ID的辅助函数
pub fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

/// 从请求头提取 trace_id（route handler 构造 RequestContext 用）
///
/// - header 存在且非空且非 "invalid" 占位 → 返回 header 值
/// - 其他情况 → 返回 "unknown"
pub fn extract_trace_id(headers: &HeaderMap) -> String {
    extract_request_id(headers)
        .filter(|s| s != "invalid" && !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with(trace: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(REQUEST_ID_HEADER, HeaderValue::from_str(trace).unwrap());
        h
    }

    #[test]
    fn extract_trace_id_returns_header_value_when_present() {
        let headers = headers_with("abc-123");
        assert_eq!(extract_trace_id(&headers), "abc-123");
    }

    #[test]
    fn extract_trace_id_returns_uuid_format_value() {
        let headers = headers_with("550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(
            extract_trace_id(&headers),
            "550e8400-e29b-41d4-a716-446655440000"
        );
    }

    #[test]
    fn extract_trace_id_falls_back_to_unknown_when_missing() {
        let headers = HeaderMap::new();
        assert_eq!(extract_trace_id(&headers), "unknown");
    }

    #[test]
    fn extract_trace_id_falls_back_to_unknown_when_value_is_invalid_placeholder() {
        let headers = headers_with("invalid");
        assert_eq!(extract_trace_id(&headers), "unknown");
    }

    #[test]
    fn extract_trace_id_falls_back_to_unknown_when_value_is_empty() {
        let headers = headers_with("");
        assert_eq!(extract_trace_id(&headers), "unknown");
    }
}
