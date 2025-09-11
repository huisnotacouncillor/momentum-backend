use redis::{AsyncCommands, RedisResult};
use serde::{Deserialize, Serialize};

use uuid::Uuid;
use crate::db::models::auth::{AuthUser, UserProfile, UserBasicInfo};
use crate::error::AppError;

/// 用户缓存键前缀
const USER_CACHE_PREFIX: &str = "user:";
const USER_PROFILE_CACHE_PREFIX: &str = "user_profile:";
const USER_WORKSPACE_CACHE_PREFIX: &str = "user_workspace:";

/// 缓存过期时间（秒）
const USER_CACHE_TTL: u64 = 3600; // 1小时
const USER_PROFILE_CACHE_TTL: u64 = 1800; // 30分钟
const USER_WORKSPACE_CACHE_TTL: u64 = 7200; // 2小时

/// 用户缓存管理器
pub struct UserCache {
    redis_client: redis::Client,
}

impl UserCache {
    /// 创建新的用户缓存管理器
    pub fn new(redis_url: &str) -> Result<Self, AppError> {
        let redis_client = redis::Client::open(redis_url)
            .map_err(|e| AppError::Internal(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self { redis_client })
    }

    /// 获取Redis连接
    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, AppError> {
        self.redis_client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))
    }

    /// 缓存用户基本信息
    pub async fn cache_user(&self, user: &AuthUser) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}{}", USER_CACHE_PREFIX, user.id);

        let user_json = serde_json::to_string(user)
            .map_err(|e| AppError::Internal(format!("Failed to serialize user: {}", e)))?;

        let _: () = conn.set_ex(&key, user_json, USER_CACHE_TTL)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cache user: {}", e)))?;

        Ok(())
    }

    /// 获取缓存的用户基本信息
    pub async fn get_user(&self, user_id: Uuid) -> Result<Option<AuthUser>, AppError> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}{}", USER_CACHE_PREFIX, user_id);

        let user_json: Option<String> = conn.get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get cached user: {}", e)))?;

        match user_json {
            Some(json) => {
                let user = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Failed to deserialize user: {}", e)))?;
                Ok(Some(user))
            }
            None => Ok(None),
        }
    }

    /// 缓存用户详细资料
    pub async fn cache_user_profile(&self, profile: &UserProfile) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}{}", USER_PROFILE_CACHE_PREFIX, profile.id);

        let profile_json = serde_json::to_string(profile)
            .map_err(|e| AppError::Internal(format!("Failed to serialize user profile: {}", e)))?;

        let _: () = conn.set_ex(&key, profile_json, USER_PROFILE_CACHE_TTL)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cache user profile: {}", e)))?;

        Ok(())
    }

    /// 获取缓存的用户详细资料
    pub async fn get_user_profile(&self, user_id: Uuid) -> Result<Option<UserProfile>, AppError> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}{}", USER_PROFILE_CACHE_PREFIX, user_id);

        let profile_json: Option<String> = conn.get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get cached user profile: {}", e)))?;

        match profile_json {
            Some(json) => {
                let profile = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Failed to deserialize user profile: {}", e)))?;
                Ok(Some(profile))
            }
            None => Ok(None),
        }
    }

    /// 缓存用户当前工作空间ID
    pub async fn cache_user_workspace(&self, user_id: Uuid, workspace_id: Option<Uuid>) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}{}", USER_WORKSPACE_CACHE_PREFIX, user_id);

        let workspace_json = serde_json::to_string(&workspace_id)
            .map_err(|e| AppError::Internal(format!("Failed to serialize workspace ID: {}", e)))?;

        let _: () = conn.set_ex(&key, workspace_json, USER_WORKSPACE_CACHE_TTL)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to cache user workspace: {}", e)))?;

        Ok(())
    }

    /// 获取缓存的用户当前工作空间ID
    pub async fn get_user_workspace(&self, user_id: Uuid) -> Result<Option<Option<Uuid>>, AppError> {
        let mut conn = self.get_connection().await?;
        let key = format!("{}{}", USER_WORKSPACE_CACHE_PREFIX, user_id);

        let workspace_json: Option<String> = conn.get(&key)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get cached user workspace: {}", e)))?;

        match workspace_json {
            Some(json) => {
                let workspace_id = serde_json::from_str(&json)
                    .map_err(|e| AppError::Internal(format!("Failed to deserialize workspace ID: {}", e)))?;
                Ok(Some(workspace_id))
            }
            None => Ok(None),
        }
    }

    /// 删除用户相关的所有缓存
    pub async fn invalidate_user_cache(&self, user_id: Uuid) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;

        let keys = vec![
            format!("{}{}", USER_CACHE_PREFIX, user_id),
            format!("{}{}", USER_PROFILE_CACHE_PREFIX, user_id),
            format!("{}{}", USER_WORKSPACE_CACHE_PREFIX, user_id),
        ];

        for key in keys {
            let _: RedisResult<i32> = conn.del(&key).await;
        }

        Ok(())
    }

    /// 批量缓存用户基本信息
    pub async fn cache_users_batch(&self, users: &[UserBasicInfo]) -> Result<(), AppError> {
        let mut conn = self.get_connection().await?;

        for user in users {
            let key = format!("{}{}", USER_CACHE_PREFIX, user.id);
            let user_json = serde_json::to_string(user)
                .map_err(|e| AppError::Internal(format!("Failed to serialize user: {}", e)))?;

            let _: RedisResult<()> = conn.set_ex(&key, user_json, USER_CACHE_TTL).await;
        }

        Ok(())
    }

    /// 检查缓存连接状态
    pub async fn health_check(&self) -> Result<bool, AppError> {
        let mut conn = self.get_connection().await?;

        let pong: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Redis health check failed: {}", e)))?;

        Ok(pong == "PONG")
    }

    /// 获取缓存统计信息
    pub async fn get_cache_stats(&self) -> Result<CacheStats, AppError> {
        let mut conn = self.get_connection().await?;

        // 获取Redis信息
        let info: String = redis::cmd("INFO")
            .arg("memory")
            .query_async(&mut conn)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get Redis info: {}", e)))?;

        // 计算用户相关键的数量
        let user_keys: Vec<String> = conn.keys(format!("{}*", USER_CACHE_PREFIX))
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get user keys: {}", e)))?;

        let profile_keys: Vec<String> = conn.keys(format!("{}*", USER_PROFILE_CACHE_PREFIX))
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get profile keys: {}", e)))?;

        let workspace_keys: Vec<String> = conn.keys(format!("{}*", USER_WORKSPACE_CACHE_PREFIX))
            .await
            .map_err(|e| AppError::Internal(format!("Failed to get workspace keys: {}", e)))?;

        Ok(CacheStats {
            user_cache_count: user_keys.len(),
            profile_cache_count: profile_keys.len(),
            workspace_cache_count: workspace_keys.len(),
            redis_info: info,
        })
    }
}

/// 缓存统计信息
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheStats {
    pub user_cache_count: usize,
    pub profile_cache_count: usize,
    pub workspace_cache_count: usize,
    pub redis_info: String,
}

/// 缓存配置
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub redis_url: String,
    pub user_ttl: u64,
    pub profile_ttl: u64,
    pub workspace_ttl: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            user_ttl: USER_CACHE_TTL,
            profile_ttl: USER_PROFILE_CACHE_TTL,
            workspace_ttl: USER_WORKSPACE_CACHE_TTL,
        }
    }
}