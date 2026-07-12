use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppErrorResponse;
use crate::middleware::request_tracking::extract_trace_id;
use crate::state::AppState;
use axum::http::HeaderMap;
use momentum_core::db::models::{
    api::ApiResponse,
    auth::{LoginRequest, RegisterRequest},
};
use momentum_core::services::auth_service::AuthService;
use momentum_core::services::context::RequestContext;
use crate::middleware::auth::AuthUserInfo;
use crate::validation::ValidatedJson;

// Re-export types from momentum_core for backward compatibility
pub use momentum_core::services::auth::types::UpdateProfileRequest;

#[derive(Deserialize)]
pub struct SwitchWorkspaceRequest {
    pub workspace_id: Uuid,
}

// 用户注册
pub async fn register(
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<RegisterRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match AuthService::register(&mut conn, &payload, &state.asset_helper, state.bcrypt_cost) {
        Ok(login_response) => {
            let response = ApiResponse::created(login_response, "User registered successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 用户登录
pub async fn login(
    State(state): State<Arc<AppState>>,
    ValidatedJson(payload): ValidatedJson<LoginRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    match AuthService::login(&mut conn, &payload, &state.asset_helper) {
        Ok(login_response) => {
            let response = ApiResponse::success(login_response, "Login successful");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 获取用户资料
pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap_or_default(),
        idempotency_key: None,
        trace_id: extract_trace_id(&headers),
    };

    match AuthService::get_profile(&mut conn, &ctx, &state.asset_helper) {
        Ok(profile) => {
            let response = ApiResponse::success(profile, "Profile retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 更新用户资料
pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Json(payload): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap_or_default(),
        idempotency_key: None,
        trace_id: extract_trace_id(&headers),
    };

    match AuthService::update_profile(&mut conn, &ctx, &payload, &state.asset_helper) {
        Ok(profile) => {
            let response = ApiResponse::success(profile, "Profile updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 切换工作空间
pub async fn switch_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
    Json(payload): Json<SwitchWorkspaceRequest>,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap_or_default(),
        idempotency_key: None,
        trace_id: extract_trace_id(&headers),
    };

    match AuthService::switch_workspace(&mut conn, &ctx, payload.workspace_id) {
        Ok(user) => {
            let response = ApiResponse::success(user, "Workspace switched successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

// 用户登出
pub async fn logout(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
    headers: HeaderMap,
) -> impl IntoResponse {
    let mut conn = match state.db.get() {
        Ok(conn) => conn,
        Err(_) => {
            let response = ApiResponse::<()>::internal_error("Database connection failed");
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
        }
    };

    let ctx = RequestContext {
        user_id: auth_info.user.id,
        workspace_id: auth_info.current_workspace_id.unwrap_or_default(),
        idempotency_key: None,
        trace_id: extract_trace_id(&headers),
    };

    // 使所有会话失效
    if let Err(err) = AuthService::logout(&mut conn, &ctx) {
        return AppErrorResponse(err).into_response();
    }

    // 清除 Redis 中的所有用户相关缓存
    let user_id = ctx.user_id;

    // 清除用户缓存的各个键
    if let Ok(mut redis_conn) = state.redis.get_multiplexed_async_connection().await {
        use redis::AsyncCommands;

        // 定义所有需要清除的键
        let cache_keys = vec![
            format!("user:{}", user_id),           // 用户基本信息
            format!("user_profile:{}", user_id),   // 用户详细资料
            format!("user_workspace:{}", user_id), // 用户工作空间
        ];

        // 批量删除缓存键
        for key in cache_keys {
            let _: Result<(), redis::RedisError> = redis_conn.del(&key).await;
        }

        tracing::info!("Cleared Redis cache for user {} on logout", user_id);
    } else {
        tracing::warn!("Failed to get Redis connection for cache cleanup on logout");
        // 即使 Redis 清理失败，登出操作仍然成功（数据库会话已失效）
    }

    let response = ApiResponse::<()>::success((), "Logout successful");
    (StatusCode::OK, Json(response)).into_response()
}
