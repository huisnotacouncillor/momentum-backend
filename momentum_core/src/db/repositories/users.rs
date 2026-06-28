use diesel::prelude::*;
use uuid::Uuid;

use crate::db::models::auth::{NewUser, NewUserCredential, User};
use crate::error::AppError;
use crate::schema::{user_credentials, users};

/// NewGitHubUser struct for creating a user from GitHub OAuth
#[derive(Debug, Clone)]
pub struct NewGitHubUser {
    pub github_id: String,
    pub username: String,
    pub email: Option<String>,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

pub struct UserRepo;

impl UserRepo {
    /// 通过 OAuth provider 和 oauth_user_id 查找用户
    pub fn find_by_oauth(
        conn: &mut PgConnection,
        provider: &str,
        oauth_user_id: &str,
    ) -> Result<Option<User>, AppError> {
        // 首先通过 user_credentials 找到 user_id
        let user_id: Option<Uuid> = user_credentials::table
            .filter(user_credentials::credential_type.eq("oauth"))
            .filter(user_credentials::oauth_provider_id.eq(provider))
            .filter(user_credentials::oauth_user_id.eq(oauth_user_id))
            .select(user_credentials::user_id)
            .first(conn)
            .optional()
            .map_err(|e| AppError::internal(format!("Failed to find oauth user: {}", e)))?;

        match user_id {
            Some(uid) => {
                // 然后通过 user_id 找到完整的用户信息
                let user = users::table
                    .filter(users::id.eq(uid))
                    .first(conn)
                    .optional()
                    .map_err(|e| AppError::internal(format!("Failed to find user: {}", e)))?;
                Ok(user)
            }
            None => Ok(None),
        }
    }

    /// 从 GitHub OAuth 创建新用户
    pub fn create_from_github(
        conn: &mut PgConnection,
        new_github_user: &NewGitHubUser,
    ) -> Result<User, AppError> {
        // 使用 diesel 的 transaction 确保原子性
        conn.transaction(|conn| {
            // 1. 创建用户
            let new_user = NewUser {
                email: new_github_user.email.clone().unwrap_or_else(|| format!("{}@github.local", new_github_user.username)),
                username: new_github_user.username.clone(),
                name: new_github_user.name.clone().unwrap_or_else(|| new_github_user.username.clone()),
                avatar_url: new_github_user.avatar_url.clone(),
            };

            let user: User = diesel::insert_into(users::table)
                .values(&new_user)
                .get_result(conn)
                .map_err(|e| AppError::internal(format!("Failed to create user: {}", e)))?;

            // 2. 创建 OAuth 凭证
            let new_credential = NewUserCredential {
                user_id: user.id,
                credential_type: "oauth".to_string(),
                credential_hash: None,
                oauth_provider_id: Some("github".to_string()),
                oauth_user_id: Some(new_github_user.github_id.clone()),
                is_primary: true,
            };

            diesel::insert_into(user_credentials::table)
                .values(&new_credential)
                .execute(conn)
                .map_err(|e| AppError::internal(format!("Failed to create oauth credential: {}", e)))?;

            Ok(user)
        })
    }
}
