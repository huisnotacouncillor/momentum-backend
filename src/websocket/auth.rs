use crate::{
    db::{DbPool, models::AuthUser},
    middleware::auth::{AuthConfig, AuthService},
};
use axum::{extract::Query, http::StatusCode};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub username: String,
    pub exp: u64,
    pub iat: u64,
    pub jti: String,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub current_workspace_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct WebSocketAuthQuery {
    pub token: Option<String>,
}

pub struct WebSocketAuth;

impl WebSocketAuth {
    /// 验证WebSocket连接的JWT token
    pub async fn authenticate_websocket(
        pool: Arc<DbPool>,
        token: &str,
    ) -> Result<AuthenticatedUser, WebSocketAuthError> {
        // 使用现有的认证服务
        let auth_service = AuthService::new(AuthConfig::default());

        // 验证JWT token
        let claims = auth_service.verify_token(token).map_err(|e| {
            tracing::error!("JWT validation failed: {}", e);
            WebSocketAuthError::InvalidToken
        })?;

        let user_id = claims.sub;

        // 从数据库获取用户信息
        let auth_user = get_user_by_id(&pool, user_id)
            .await
            .map_err(|_| WebSocketAuthError::UserNotFound)?;

        // 获取用户的当前工作区
        let current_workspace_id = get_user_current_workspace(&pool, user_id).await.ok();

        Ok(AuthenticatedUser {
            user_id: auth_user.id,
            username: auth_user.username,
            email: auth_user.email,
            name: auth_user.name,
            avatar_url: auth_user.avatar_url,
            current_workspace_id,
        })
    }

    /// 从查询参数中提取并验证token
    pub async fn extract_and_validate_token(
        pool: Arc<DbPool>,
        query: Query<WebSocketAuthQuery>,
    ) -> Result<AuthenticatedUser, WebSocketAuthError> {
        let token = query
            .token
            .as_ref()
            .ok_or(WebSocketAuthError::MissingToken)?;

        Self::authenticate_websocket(pool, token).await
    }

    /// 从URL参数或Header中提取token
    pub fn extract_token_from_params(query_params: &HashMap<String, String>) -> Option<String> {
        // 首先尝试从查询参数获取
        if let Some(token) = query_params.get("token") {
            return Some(token.clone());
        }

        // 尝试从authorization参数获取
        if let Some(auth) = query_params.get("authorization") {
            if let Some(token) = auth.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }

        None
    }

    /// 验证token格式
    pub fn validate_token_format(token: &str) -> bool {
        // 基本的JWT格式验证（三个部分用.分隔）
        let parts: Vec<&str> = token.split('.').collect();
        parts.len() == 3 && parts.iter().all(|part| !part.is_empty())
    }

    /// 检查token是否过期
    pub fn is_token_expired(claims: &Claims) -> bool {
        let now = chrono::Utc::now().timestamp() as u64;
        claims.exp < now
    }

    /// 生成错误响应
    pub fn error_response(error: WebSocketAuthError) -> (StatusCode, &'static str) {
        match error {
            WebSocketAuthError::MissingToken => {
                (StatusCode::UNAUTHORIZED, "Missing authentication token")
            }
            WebSocketAuthError::InvalidToken => {
                (StatusCode::UNAUTHORIZED, "Invalid authentication token")
            }
            WebSocketAuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, "Token has expired"),
            WebSocketAuthError::UserNotFound => (StatusCode::NOT_FOUND, "User not found"),
            WebSocketAuthError::InvalidUserId => {
                (StatusCode::BAD_REQUEST, "Invalid user ID format")
            }
            WebSocketAuthError::DatabaseError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error")
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum WebSocketAuthError {
    MissingToken,
    InvalidToken,
    ExpiredToken,
    UserNotFound,
    InvalidUserId,
    DatabaseError,
}

impl std::fmt::Display for WebSocketAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebSocketAuthError::MissingToken => write!(f, "Missing authentication token"),
            WebSocketAuthError::InvalidToken => write!(f, "Invalid authentication token"),
            WebSocketAuthError::ExpiredToken => write!(f, "Token has expired"),
            WebSocketAuthError::UserNotFound => write!(f, "User not found"),
            WebSocketAuthError::InvalidUserId => write!(f, "Invalid user ID format"),
            WebSocketAuthError::DatabaseError => write!(f, "Database error"),
        }
    }
}

impl std::error::Error for WebSocketAuthError {}

async fn get_user_by_id(
    pool: &Arc<DbPool>,
    user_id: Uuid,
) -> Result<AuthUser, diesel::result::Error> {
    use crate::schema::users::dsl::*;

    let mut conn = pool.get().expect("Failed to get DB connection");

    let user = users
        .filter(id.eq(user_id))
        .filter(is_active.eq(true))
        .select(crate::db::models::User::as_select())
        .first(&mut conn)?;

    Ok(AuthUser {
        id: user.id,
        email: user.email,
        username: user.username,
        name: user.name,
        avatar_url: user.avatar_url,
    })
}

async fn get_user_current_workspace(
    pool: &Arc<DbPool>,
    user_id: Uuid,
) -> Result<Uuid, diesel::result::Error> {
    use crate::schema::users::dsl::*;

    let mut conn = pool.get().expect("Failed to get DB connection");

    let workspace_id = users
        .filter(id.eq(user_id))
        .filter(is_active.eq(true))
        .select(current_workspace_id)
        .first::<Option<Uuid>>(&mut conn)?
        .ok_or_else(|| diesel::result::Error::NotFound)?;

    Ok(workspace_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_token_format() {
        // 有效的JWT格式
        let valid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        assert!(WebSocketAuth::validate_token_format(valid_token));

        // 无效格式
        let invalid_token = "invalid.token";
        assert!(!WebSocketAuth::validate_token_format(invalid_token));

        let empty_token = "";
        assert!(!WebSocketAuth::validate_token_format(empty_token));
    }

    #[test]
    fn test_extract_token_from_params() {
        let mut params = HashMap::new();
        params.insert("token".to_string(), "test_token".to_string());

        let token = WebSocketAuth::extract_token_from_params(&params);
        assert_eq!(token, Some("test_token".to_string()));

        // 测试Bearer token
        let mut auth_params = HashMap::new();
        auth_params.insert(
            "authorization".to_string(),
            "Bearer test_bearer_token".to_string(),
        );

        let bearer_token = WebSocketAuth::extract_token_from_params(&auth_params);
        assert_eq!(bearer_token, Some("test_bearer_token".to_string()));
    }

    #[test]
    fn test_is_token_expired() {
        let now = chrono::Utc::now().timestamp() as u64;

        // 未过期的token
        let valid_claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            username: "test_user".to_string(),
            exp: now + 3600, // 1小时后过期
            iat: now,
            jti: Uuid::new_v4().to_string(),
        };
        assert!(!WebSocketAuth::is_token_expired(&valid_claims));

        // 已过期的token
        let expired_claims = Claims {
            sub: Uuid::new_v4(),
            email: "test@example.com".to_string(),
            username: "test_user".to_string(),
            exp: now - 3600, // 1小时前过期
            iat: now - 7200,
            jti: Uuid::new_v4().to_string(),
        };
        assert!(WebSocketAuth::is_token_expired(&expired_claims));
    }
}
