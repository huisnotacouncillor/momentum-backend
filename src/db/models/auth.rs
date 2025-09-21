use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::StatusCode;
use axum::http::request::Parts;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;
use crate::validation::rules::{validate_password_strength, validate_username_format};

// User models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone, Debug)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub name: String,
    pub email: String,
    pub username: String,
    pub avatar_url: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub id: Uuid,
    pub current_workspace_id: Option<Uuid>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::users)]
pub struct NewUser {
    pub email: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

// User Credential models
#[derive(Queryable, Selectable, Serialize, Deserialize, Clone)]
#[diesel(table_name = crate::schema::user_credentials)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserCredential {
    pub id: i32,
    pub user_id: Uuid,
    pub credential_type: String,
    pub credential_hash: Option<String>,
    pub oauth_provider_id: Option<String>,
    pub oauth_user_id: Option<String>,
    pub is_primary: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::user_credentials)]
pub struct NewUserCredential {
    pub user_id: Uuid,
    pub credential_type: String,
    pub credential_hash: Option<String>,
    pub oauth_provider_id: Option<String>,
    pub oauth_user_id: Option<String>,
    pub is_primary: bool,
}

// Authentication DTOs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(auth_info) = parts
            .extensions
            .get::<crate::middleware::auth::AuthUserInfo>()
        {
            Ok(auth_info.user.clone())
        } else {
            Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(custom(function = "validate_username_format"))]
    pub username: String,

    #[validate(length(min = 1, max = 100, message = "Name must be between 1 and 100 characters"))]
    pub name: String,

    #[validate(custom(function = "validate_password_strength"))]
    pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required"))]
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub user: AuthUser,
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct UserBasicInfo {
    pub id: Uuid,
    pub name: String,
    pub username: String,
    pub email: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct UserProfile {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub current_workspace_id: Option<Uuid>,
    pub workspaces: Vec<super::workspace::WorkspaceInfo>,
    pub teams: Vec<super::team::TeamInfo>,
}

// 为用户模型添加资源 URL 处理方法
impl User {
    /// 获取处理后的头像 URL
    /// 如果 avatar_url 存在，则使用 AssetUrlHelper 处理；否则返回 None
    pub fn get_processed_avatar_url(&self, asset_helper: &crate::utils::AssetUrlHelper) -> Option<String> {
        self.avatar_url.as_ref().map(|url| asset_helper.process_url(url))
    }

    /// 优化的头像 URL 处理方法，避免不必要的字符串分配
    pub fn get_processed_avatar_url_ref<'a>(&'a self, asset_helper: &'a crate::utils::AssetUrlHelper) -> Option<std::borrow::Cow<'a, str>> {
        self.avatar_url.as_ref().map(|url| asset_helper.process_url_ref(url))
    }
}

impl AuthUser {
    /// 获取处理后的头像 URL
    /// 如果 avatar_url 存在，则使用 AssetUrlHelper 处理；否则返回 None
    pub fn get_processed_avatar_url(&self, asset_helper: &crate::utils::AssetUrlHelper) -> Option<String> {
        self.avatar_url.as_ref().map(|url| asset_helper.process_url(url))
    }
}

impl UserBasicInfo {
    /// 获取处理后的头像 URL
    /// 如果 avatar_url 存在，则使用 AssetUrlHelper 处理；否则返回 None
    pub fn get_processed_avatar_url(&self, asset_helper: &crate::utils::AssetUrlHelper) -> Option<String> {
        self.avatar_url.as_ref().map(|url| asset_helper.process_url(url))
    }
}

impl UserProfile {
    /// 获取处理后的头像 URL
    /// 如果 avatar_url 存在，则使用 AssetUrlHelper 处理；否则返回 None
    pub fn get_processed_avatar_url(&self, asset_helper: &crate::utils::AssetUrlHelper) -> Option<String> {
        self.avatar_url.as_ref().map(|url| asset_helper.process_url(url))
    }
}
