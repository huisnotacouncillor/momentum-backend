use crate::db::{DbPool, models::AuthUser};
use axum::{
    extract::State,
    http::{Request, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

pub struct AuthConfig {
    pub jwt_secret: String,
    pub jwt_expiration: Duration,
    pub refresh_expiration: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key".to_string()),
            jwt_expiration: Duration::from_secs(3600), // 1 hour
            refresh_expiration: Duration::from_secs(7 * 24 * 3600), // 7 days
        }
    }
}

pub struct AuthService {
    config: AuthConfig,
}

impl AuthService {
    pub fn new(config: AuthConfig) -> Self {
        Self { config }
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
) -> Result<Response, StatusCode> {
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|auth_header| auth_header.to_str().ok())
        .and_then(|auth_str| {
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        });

    let token = auth_header.ok_or(StatusCode::UNAUTHORIZED)?;

    // 创建认证服务实例
    let auth_service: AuthService = AuthService::new(AuthConfig::default());

    // 验证token
    let claims = auth_service
        .verify_token(&token)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 从数据库获取用户信息
    let user = get_user_by_id(&pool, claims.sub)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // 将用户信息添加到请求扩展中
    request.extensions_mut().insert(user);

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
            if auth_str.starts_with("Bearer ") {
                Some(auth_str[7..].to_string())
            } else {
                None
            }
        });

    if let Some(token) = &auth_header {
        let auth_service = AuthService::new(AuthConfig::default());

        if let Ok(claims) = auth_service.verify_token(&token) {
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
) -> Result<AuthUser, diesel::result::Error> {
    use crate::schema::users::dsl::*;
    use diesel::prelude::*;

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
