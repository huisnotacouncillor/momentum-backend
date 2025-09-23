use crate::AppState;
use crate::db::models::api::ApiResponse;
use crate::middleware::auth::AuthUserInfo;
use crate::services::context::RequestContext;
use crate::services::auth_service::AuthService;
use axum::{
    Json,
    extract::{State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
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

    match AuthService::update_profile(&mut conn, &ctx, &crate::routes::auth::UpdateProfileRequest {
        name: payload.name.clone(),
        username: payload.username.clone(),
        email: payload.email.clone(),
        avatar_url: payload.avatar_url.clone(),
    }) {
        Ok(profile) => {
            let response = ApiResponse::success(profile, "Profile updated successfully");
            (StatusCode::OK, Json(response)).into_response()
        }
        Err(err) => err.into_response(),
    }
}