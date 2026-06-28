use reqwest::Client;
use serde::Deserialize;

use crate::db::models::auth::User;
use crate::db::repositories::oauth::OAuthRepo;
use crate::db::repositories::users::{UserRepo, NewGitHubUser};
use crate::db::DbPool;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct GitHubUserInfo {
    pub id: i64,
    pub login: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GitHubTokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: String,
}

pub struct OAuthService {
    pool: DbPool,
    http_client: Client,
}

impl OAuthService {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            http_client: Client::new(),
        }
    }

    /// 生成 GitHub 授权 URL
    pub fn get_github_auth_url(&self, state: &str) -> Result<String, AppError> {
        let mut conn = self.pool.get().map_err(|e| AppError::internal(e.to_string()))?;

        let provider = OAuthRepo::find_by_provider_name(&mut conn, "github")?
            .ok_or_else(|| AppError::not_found("GitHub OAuth provider not configured"))?;

        let scope = provider.scope.unwrap_or_else(|| "read:user user:email".to_string());
        let redirect_uri = provider.redirect_uri.unwrap_or_else(|| "".to_string());

        let auth_url = format!(
            "{}?client_id={}&redirect_uri={}&scope={}&state={}",
            provider.auth_url,
            provider.client_id,
            redirect_uri,
            scope,
            state
        );

        Ok(auth_url)
    }

    /// 交换授权码获取 access_token
    pub async fn exchange_code_for_token(&self, code: &str) -> Result<String, AppError> {
        let mut conn = self.pool.get().map_err(|e| AppError::internal(e.to_string()))?;

        let provider = OAuthRepo::find_by_provider_name(&mut conn, "github")?
            .ok_or_else(|| AppError::not_found("GitHub OAuth provider not configured"))?;

        let redirect_uri = provider.redirect_uri.unwrap_or_else(|| "".to_string());

        let params = [
            ("client_id", provider.client_id.as_str()),
            ("client_secret", provider.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri.as_str()),
        ];

        let response = self
            .http_client
            .post(&provider.token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to exchange code: {}", e)))?;

        let token_response: GitHubTokenResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse token response: {}", e)))?;

        Ok(token_response.access_token)
    }

    /// 获取 GitHub 用户信息
    pub async fn get_github_user_info(&self, access_token: &str) -> Result<GitHubUserInfo, AppError> {
        let response = self
            .http_client
            .get("https://api.github.com/user")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("User-Agent", "Momentum-OAuth")
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Failed to get user info: {}", e)))?;

        let user_info: GitHubUserInfo = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse user info: {}", e)))?;

        Ok(user_info)
    }

    /// 处理 GitHub OAuth 回调
    pub async fn handle_github_callback(&self, code: &str) -> Result<AuthResult, AppError> {
        // 1. 交换 code 获取 access_token
        let access_token = self.exchange_code_for_token(code).await?;

        // 2. 获取 GitHub 用户信息
        let github_user = self.get_github_user_info(&access_token).await?;

        let mut conn = self.pool.get().map_err(|e| AppError::internal(e.to_string()))?;

        // 3. 检查用户是否已存在
        let user = UserRepo::find_by_oauth(&mut conn, "github", &github_user.id.to_string())?;

        if let Some(user) = user {
            return Ok(AuthResult::ExistingUser(user));
        }

        // 4. 创建新用户
        let new_user = NewGitHubUser {
            github_id: github_user.id.to_string(),
            username: github_user.login.clone(),
            email: github_user.email,
            name: github_user.name,
            avatar_url: github_user.avatar_url,
        };

        let user = UserRepo::create_from_github(&mut conn, &new_user)?;

        Ok(AuthResult::NewUser(user))
    }
}

pub enum AuthResult {
    ExistingUser(User),
    NewUser(User),
}
