use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::AppErrorResponse;
use crate::state::AppState;
use momentum_core::config::Config;
use momentum_core::db::models::auth::AuthUser;
use momentum_core::error::AppError;
use momentum_core::services::oauth_service::{AuthResult, OAuthService};
use momentum_core::services::jwt::JwtService;

#[derive(Debug, Deserialize)]
pub struct GitHubCallbackQuery {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user_id: Uuid,
    pub is_new_user: bool,
}

pub async fn github_callback(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GitHubCallbackQuery>,
) -> impl IntoResponse {
    let oauth_service = OAuthService::new(state.db.clone());

    let result = match oauth_service.handle_github_callback(&params.code).await {
        Ok(result) => result,
        Err(err) => return AppErrorResponse(err).into_response(),
    };

    let user_id = match &result {
        AuthResult::ExistingUser(u) => u.id,
        AuthResult::NewUser(u) => u.id,
    };

    let is_new_user = matches!(result, AuthResult::NewUser(_));

    // Get user for token generation
    let user = match &result {
        AuthResult::ExistingUser(u) => u,
        AuthResult::NewUser(u) => u,
    };

    // Create an AuthUser for JWT generation
    let auth_user = AuthUser {
        id: user.id,
        email: user.email.clone(),
        username: user.username.clone(),
        name: user.name.clone(),
        avatar_url: user.avatar_url.clone(),
    };

    // Get JWT config from environment
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => return AppErrorResponse(err).into_response(),
    };

    let jwt_service = JwtService::from_config(&config);

    let access_token = match jwt_service.generate_access_token(&auth_user) {
        Ok(token) => token,
        Err(err) => return AppErrorResponse(AppError::Jwt(err)).into_response(),
    };

    let refresh_token = match jwt_service.generate_refresh_token(user_id) {
        Ok(token) => token,
        Err(err) => return AppErrorResponse(AppError::Jwt(err)).into_response(),
    };

    let response = AuthResponse {
        access_token,
        refresh_token,
        user_id,
        is_new_user,
    };

    (StatusCode::OK, Json(response)).into_response()
}

pub fn routes() -> Router<Arc<AppState>> {
    use axum::routing::get;

    Router::new()
        .route("/api/auth/github/callback", get(github_callback))
}