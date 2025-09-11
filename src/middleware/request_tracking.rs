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
pub async fn request_tracking_middleware<B>(
    mut request: Request<B>,
    next: Next<B>,
) -> Response {
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

/// 性能监控中间件
/// 专门用于监控API性能指标
pub async fn performance_monitoring_middleware<B>(
    request: Request<B>,
    next: Next<B>,
) -> Response {
    let start_time = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().path().to_string();

    // 处理请求
    let response = next.run(request).await;

    // 计算性能指标
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis();
    let status_code = response.status().as_u16();

    // 记录性能指标
    info!(
        method = %method,
        uri = %uri,
        status_code = %status_code,
        duration_ms = %duration_ms,
        "API performance metrics"
    );

    // 根据不同的性能阈值记录不同级别的日志
    match duration_ms {
        0..=100 => {
            // 快速响应，debug级别
            tracing::debug!(
                method = %method,
                uri = %uri,
                duration_ms = %duration_ms,
                "Fast response"
            );
        }
        101..=500 => {
            // 正常响应，info级别
            info!(
                method = %method,
                uri = %uri,
                duration_ms = %duration_ms,
                "Normal response"
            );
        }
        501..=1000 => {
            // 较慢响应，warn级别
            warn!(
                method = %method,
                uri = %uri,
                duration_ms = %duration_ms,
                "Slow response"
            );
        }
        _ => {
            // 非常慢的响应，error级别
            tracing::error!(
                method = %method,
                uri = %uri,
                duration_ms = %duration_ms,
                "Very slow response"
            );
        }
    }

    response
}

/// 从请求头中提取请求ID的辅助函数
pub fn extract_request_id(headers: &HeaderMap) -> Option<String> {
    headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}