use momentum_core::db::models::{ApiResponse, ErrorDetail, User};
use momentum_core::db::{DbPool, models::AuthUser};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: uuid::Uuid, // user_id
    pub email: String,
    pub username: String,
    pub exp: u64,    // expiration time
    pub iat: u64,    // issued at
    pub jti: String, // JWT ID
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshClaims {
    pub sub: uuid::Uuid, // user_id
    pub exp: u64,        // expiration time
    pub iat: u64,        // issued at
    pub jti: String,     // JWT ID
}

#[derive(Clone)]
pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration: Duration,
    pub refresh_expiration: Duration,
}

// P2.2 修复：AuthConfig 不再提供 Default 实现，强制从配置创建
// 旧实现会回退到硬编码 "your-secret-key"，导致生产环境密钥泄漏
//
// 新方式：通过 `AuthConfig::from_config(&core_config)` 从主配置创建
impl AuthConfig {
    /// 从 core 配置创建 AuthConfig
    pub fn from_config(core_config: &momentum_core::config::Config) -> Self {
        Self {
            jwt_secret: core_config.jwt_secret.clone(),
            jwt_expiration: Duration::from_secs(core_config.jwt_access_token_expires_in),
            refresh_expiration: Duration::from_secs(core_config.jwt_refresh_token_expires_in),
        }
    }

    /// 从环境变量创建（带严格验证）
    ///
    /// 如果 JWT_SECRET 未设置或等于占位符，会 panic
    pub fn from_env_strict() -> Self {
        let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            panic!(
                "JWT_SECRET environment variable is required. \
                 Set it to a secure random value (>= 32 bytes)."
            );
        });

        if secret == "your-secret-key"
            || secret == "your-super-secret-jwt-key-change-this-in-production"
            || secret.len() < 32
        {
            panic!(
                "JWT_SECRET is set to an insecure value. \
                 Use a random secret of at least 32 bytes."
            );
        }

        Self {
            jwt_secret: secret,
            jwt_expiration: Duration::from_secs(3600),
            refresh_expiration: Duration::from_secs(7 * 24 * 3600),
        }
    }
}

#[derive(Clone)]
pub struct AuthService {
    config: AuthConfig,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
    }

    /// 检查token是否需要续期
    pub fn should_refresh_token(&self, claims: &Claims) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let time_until_expiry = claims.exp.saturating_sub(now);
        // 距离过期还有15分钟时续期
        time_until_expiry <= 15 * 60
    }

    /// 检查token是否即将过期（用于客户端提示）
    pub fn is_token_expiring_soon(&self, claims: &Claims) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let time_until_expiry = claims.exp.saturating_sub(now);
        // 距离过期还有5分钟时提示
        time_until_expiry <= 5 * 60
    }

    pub fn generate_access_token(
        &self,
        user: &AuthUser,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            exp: now + self.config.jwt_expiration.as_secs(),
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_ref()),
        )
    }

    pub fn generate_refresh_token(
        &self,
        user_id: uuid::Uuid,
    ) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = RefreshClaims {
            sub: user_id,
            exp: now + self.config.refresh_expiration.as_secs(),
            iat: now,
            jti: uuid::Uuid::new_v4().to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_ref()),
        )
    }

    pub fn verify_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn verify_refresh_token(
        &self,
        token: &str,
    ) -> Result<RefreshClaims, jsonwebtoken::errors::Error> {
        let token_data = decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_ref()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }
}

pub async fn auth_middleware(
    State(pool): State<Arc<DbPool>>,
    mut request: Request<axum::body::Body>,
    next: Next<axum::body::Body>,
) -> Result<Response, Response> {
    // P2.2 修复：从请求扩展中获取预配置的 AuthConfig
    // (由 main.rs 在启动时通过 layer 注入)
    let auth_config = request
        .extensions()
        .get::<AuthConfig>()
        .cloned()
        .unwrap_or_else(|| {
            tracing::error!("AuthConfig not found in request extensions");
            AuthConfig::from_env_strict()
        });

    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| {
            auth_str
                .strip_prefix("Bearer ")
                .map(|stripped| stripped.to_string())
        });

    let token = match auth_header {
        Some(token) => token,
        None => {
            let response = ApiResponse::<()>::unauthorized("Missing authorization header");
            return Err((StatusCode::UNAUTHORIZED, Json(response)).into_response());
        }
    };

    // 创建认证服务实例（使用从请求扩展中获取的配置）
    let auth_service: AuthService = AuthService::new(auth_config);

    // 验证token
    let claims = match auth_service.verify_token(&token) {
        Ok(claims) => claims,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("Invalid or expired access token");
            return Err((StatusCode::UNAUTHORIZED, Json(response)).into_response());
        }
    };

    // 从数据库获取用户信息
    let user = match get_user_by_id(&pool, claims.sub).await {
        Ok(user) => user,
        Err(_) => {
            let response = ApiResponse::<()>::unauthorized("User not found or inactive");
            return Err((StatusCode::UNAUTHORIZED, Json(response)).into_response());
        }
    };

    // 检查用户是否有当前工作区
    if user.current_workspace_id.is_none() {
        let response = ApiResponse::<()>::error(
            400,
            "No current workspace found",
            vec![ErrorDetail {
                field: None,
                code: "NO_WORKSPACE".to_string(),
                message: "No current workspace found for user".to_string(),
            }],
        );
        return Err((StatusCode::BAD_REQUEST, Json(response)).into_response());
    }

    // 检查token是否需要续期（距离过期还有15分钟时续期）
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let time_until_expiry = claims.exp.saturating_sub(now);
    let should_refresh = time_until_expiry <= 15 * 60; // 15分钟

    // 构建认证用户信息
    let auth_user_info = AuthUserInfo {
        user: AuthUser {
            id: user.id,
            email: user.email.clone(),
            username: user.username.clone(),
            name: user.name.clone(),
            avatar_url: user.avatar_url.clone(),
        },
        current_workspace_id: user.current_workspace_id,
    };

    // 将用户信息添加到请求扩展中
    request.extensions_mut().insert(auth_user_info);

    if should_refresh {
        // 生成新的access token
        let auth_user = AuthUser {
            id: user.id,
            email: user.email,
            username: user.username,
            name: user.name,
            avatar_url: user.avatar_url,
        };

        if let Ok(new_access_token) = auth_service.generate_access_token(&auth_user) {
            // 将新token添加到响应头中
            let mut response = next.run(request).await;
            response
                .headers_mut()
                .insert("X-New-Access-Token", new_access_token.parse().unwrap());
            return Ok(response);
        }
    }

    Ok(next.run(request).await)
}

pub async fn optional_auth_middleware(
    State(pool): State<Arc<DbPool>>,
    mut request: Request<axum::body::Body>,
    next: Next<axum::body::Body>,
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| {
            auth_str
                .strip_prefix("Bearer ")
                .map(|stripped| stripped.to_string())
        });

    if let Some(token) = &auth_header {
        // P2.2 修复：从请求扩展获取配置或使用严格环境变量读取
        let auth_config = request
            .extensions()
            .get::<AuthConfig>()
            .cloned()
            .unwrap_or_else(AuthConfig::from_env_strict);
        let auth_service = AuthService::new(auth_config);

        if let Ok(claims) = auth_service.verify_token(token) {
            if let Ok(user) = get_user_by_id(&pool, claims.sub).await {
                request.extensions_mut().insert(Some(user));
            } else {
                request.extensions_mut().insert(None::<AuthUser>);
            }
        } else {
            request.extensions_mut().insert(None::<AuthUser>);
        }
    } else {
        request.extensions_mut().insert(None::<AuthUser>);
    }

    Ok(next.run(request).await)
}

async fn get_user_by_id(
    pool: &Arc<DbPool>,
    user_id: uuid::Uuid,
) -> Result<User, momentum_core::error::AppError> {
    use momentum_core::schema::users::dsl::*;
    use diesel::prelude::*;

    let mut conn = pool.get().map_err(|_| {
        momentum_core::error::AppError::ServiceUnavailable {
            message: "Database temporarily unavailable".to_string(),
        }
    })?;

    users
        .filter(id.eq(user_id))
        .filter(is_active.eq(true))
        .select(User::as_select())
        .first(&mut conn)
        .map_err(momentum_core::error::AppError::Database)
}

// 提取器，用于从请求中获取当前用户
pub async fn extract_current_user(
    axum::extract::Extension(user): axum::extract::Extension<AuthUser>,
) -> AuthUser {
    user
}

// 可选用户提取器
pub async fn extract_optional_user(
    axum::extract::Extension(user): axum::extract::Extension<Option<AuthUser>>,
) -> Option<AuthUser> {
    user
}

#[derive(Debug, Clone)]
pub struct AuthUserInfo {
    pub user: AuthUser,
    pub current_workspace_id: Option<Uuid>,
}

use axum::async_trait;
use axum::http::request::Parts;

#[async_trait]
impl<S> FromRequestParts<S> for AuthUserInfo
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        if let Some(auth_info) = parts.extensions.get::<AuthUserInfo>() {
            Ok(auth_info.clone())
        } else {
            Err((StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
        }
    }
}
