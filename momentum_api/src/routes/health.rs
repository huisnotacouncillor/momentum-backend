//! Health check endpoints
//!
//! P0 修复：提供 /health 和 /ready 端点供 Docker HEALTHCHECK 使用

use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde_json::json;
use std::sync::Arc;

/// Liveness check - 进程是否在运行
/// 不检查依赖，适合用作容器 liveness probe
pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, Json(json!({ "status": "alive" })))
}

/// Readiness check - 是否可以接受流量
/// 检查关键依赖（DB、Redis）
pub async fn readiness(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut checks = serde_json::Map::new();
    let mut all_ok = true;

    // 检查数据库
    match state.db.get() {
        Ok(_) => {
            checks.insert("database".to_string(), json!("ok"));
        }
        Err(e) => {
            all_ok = false;
            tracing::error!("Database health check failed: {}", e);
            checks.insert("database".to_string(), json!(format!("error: {}", e)));
        }
    }

    // 检查 Redis
    match state.redis.get_multiplexed_async_connection().await {
        Ok(_) => {
            checks.insert("redis".to_string(), json!("ok"));
        }
        Err(e) => {
            all_ok = false;
            tracing::error!("Redis health check failed: {}", e);
            checks.insert("redis".to_string(), json!(format!("error: {}", e)));
        }
    }

    let status = if all_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status,
        Json(json!({
            "status": if all_ok { "ready" } else { "unavailable" },
            "checks": checks,
        })),
    )
}

/// Alias for /health - 完整健康检查
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    readiness(State(state)).await
}