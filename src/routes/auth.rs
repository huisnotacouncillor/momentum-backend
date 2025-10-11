use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    AppState,
    db::models::{
        api::ApiResponse,
        auth::{LoginRequest, RegisterRequest},
    },
    middleware::auth::AuthUserInfo,
    services::auth_service::AuthService,
    services::context::RequestContext,
    validation::ValidatedJson,
};

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
}

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

    match AuthService::register(&mut conn, &payload) {
        Ok(user) => {
            let response = ApiResponse::created(user, "User registered successfully");
            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
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
        Err(err) => err.into_response(),
    }
}

// 获取用户资料
pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
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
    };

    match AuthService::get_profile(&mut conn, &ctx, &state.asset_helper) {
        Ok(profile) => {
            let response = ApiResponse::success(profile, "Profile retrieved successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

// 更新用户资料
pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
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
    };

    match AuthService::update_profile(&mut conn, &ctx, &payload, &state.asset_helper) {
        Ok(profile) => {
            let response = ApiResponse::success(profile, "Profile updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}

// 切换工作空间
pub async fn switch_workspace(
    State(state): State<Arc<AppState>>,
    auth_info: AuthUserInfo,
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
    };

    match AuthService::switch_workspace(&mut conn, &ctx, payload.workspace_id) {
        Ok(user) => {
            let response = ApiResponse::success(user, "Workspace switched successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}
