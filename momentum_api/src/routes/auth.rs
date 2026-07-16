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

    let user_id = auth_info.user.id;
    match AuthService::switch_workspace(&mut conn, &ctx, payload.workspace_id) {
        Ok(user) => {
            // Issue #12：DB 切换成功后必须立即失效 Redis 用户缓存，
            // 否则 user:{id}/user_workspace:{id} 等 key 仍然指向旧工作区，
            // 客户端拿到的是过期数据直到 TTL 自然过期。
            invalidate_user_cache(&state, user_id).await;
            let response = ApiResponse::success(user, "Workspace switched successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => AppErrorResponse(err).into_response(),
    }
}

/// Issue #12：失效用户相关 Redis 缓存键（best-effort，失败仅记录 warn）
async fn invalidate_user_cache(state: &AppState, user_id: Uuid) {
    let cache_keys = [
        format!("user:{}", user_id),
        format!("user_profile:{}", user_id),
        format!("user_workspace:{}", user_id),
    ];
    match state.redis.get_multiplexed_async_connection().await {
        Ok(mut conn) => {
            use redis::AsyncCommands;
            for key in &cache_keys {
                if let Err(e) = conn.del::<_, ()>(key).await {
                    tracing::warn!(key = %key, error = %e, "failed to invalidate user cache after workspace switch");
                }
            }
            tracing::info!(
                user_id = %user_id,
                keys = ?cache_keys,
                "Invalidated user cache after workspace switch"
            );
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "could not connect to Redis to invalidate user cache after workspace switch"
            );
        }
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

// ===== Issue #10: Refresh Token 旋转 =====

/// Refresh token 请求
#[derive(Deserialize)]
pub struct RefreshTokenHttpRequest {
    pub refresh_token: String,
}

/// `POST /auth/refresh`
///
/// 客户端提交 refresh_token，服务端：
/// 1. 在 store 中查找
/// 2. 验证未过期、未被撤销
/// 3. 旋转：旧 token 标记 Used，签发新 token
/// 4. 重放检测：见到 Used/Revoked token → 撤销整族
pub async fn refresh_token(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RefreshTokenHttpRequest>,
) -> impl IntoResponse {
    use crate::routes::refresh_token_store::RotateResult;

    // 从 header 拿 trace_id（Issue #1 兼容）
    let _trace_id = crate::middleware::request_tracking::extract_trace_id(&headers);

    match state
        .refresh_token_store
        .rotate(&payload.refresh_token, || uuid::Uuid::new_v4().to_string())
        .await
    {
        RotateResult::Success { new_token, user_id, .. } => {
            // 重新签发 access_token：复用 startup 时构造的 JwtService
            let auth_user = momentum_core::db::models::auth::AuthUser {
                id: user_id,
                email: String::new(),
                username: String::new(),
                name: String::new(),
                avatar_url: None,
            };
            let access_token = match state.jwt_service.generate_access_token(&auth_user) {
                Ok(t) => t,
                Err(_) => {
                    let response = ApiResponse::<()>::internal_error("Failed to sign access token");
                    return (StatusCode::INTERNAL_SERVER_ERROR, Json(response)).into_response();
                }
            };
            let response = ApiResponse::success(
                serde_json::json!({
                    "access_token": access_token,
                    "refresh_token": new_token,
                    "token_type": "Bearer",
                    "expires_in": 3600,
                }),
                "Token rotated",
            );
            (StatusCode::OK, Json(response)).into_response()
        }
        RotateResult::ReplayDetected { .. } => {
            tracing::warn!("Refresh token replay detected. Revoked family.");
            let response = ApiResponse::<()>::forbidden("Token replay detected; family revoked");
            (StatusCode::FORBIDDEN, Json(response)).into_response()
        }
        RotateResult::AlreadyUsed { .. } | RotateResult::Unknown => {
            let response = ApiResponse::<()>::unauthorized("Invalid refresh token");
            (StatusCode::UNAUTHORIZED, Json(response)).into_response()
        }
        RotateResult::Expired => {
            let response = ApiResponse::<()>::unauthorized("Refresh token expired");
            (StatusCode::UNAUTHORIZED, Json(response)).into_response()
        }
    }
}
