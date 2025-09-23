pub mod user_cache;
pub mod redis;

pub use user_cache::{UserCache, CacheStats, CacheConfig};

use ::redis::{AsyncCommands, RedisResult, Client, cmd};
use uuid::Uuid;
use crate::error::AppError;

/// 全局缓存操作函数
/// 这些函数提供了简化的缓存操作接口

/// 设置用户当前工作空间ID到缓存
pub async fn set_user_current_workspace_id(
    redis_client: &Client,
    user_id: Uuid,
    workspace_id: Uuid,
) -> Result<(), AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let key = format!("user_workspace:{}", user_id);
    let workspace_json = serde_json::to_string(&workspace_id)
        .map_err(|e| AppError::Internal(format!("Failed to serialize workspace ID: {}", e)))?;

    let _: () = conn.set_ex(&key, workspace_json, 7200)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to cache user workspace: {}", e)))?;

    Ok(())
}

/// 从缓存获取用户当前工作空间ID
pub async fn get_user_current_workspace_id(
    redis_client: &Client,
    user_id: Uuid,
) -> Result<Option<Uuid>, AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let key = format!("user_workspace:{}", user_id);
    let workspace_json: Option<String> = conn
        .get(&key)
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

/// 删除用户工作空间缓存
pub async fn delete_user_workspace_cache(
    redis_client: &Client,
    user_id: Uuid,
) -> Result<(), AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let key = format!("user_workspace:{}", user_id);
    let _: RedisResult<i32> = conn.del(&key).await;

    Ok(())
}

/// 缓存会话信息
pub async fn cache_session(
    redis_client: &Client,
    session_id: &str,
    user_id: Uuid,
    ttl: u64,
) -> Result<(), AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let key = format!("session:{}", session_id);
    let user_json = serde_json::to_string(&user_id)
        .map_err(|e| AppError::Internal(format!("Failed to serialize user ID: {}", e)))?;

    let _: () = conn.set_ex(&key, user_json, ttl)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to cache session: {}", e)))?;

    Ok(())
}

/// 获取会话信息
pub async fn get_session(
    redis_client: &Client,
    session_id: &str,
) -> Result<Option<Uuid>, AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let key = format!("session:{}", session_id);
    let user_json: Option<String> = conn
        .get(&key)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get cached session: {}", e)))?;

    match user_json {
        Some(json) => {
            let user_id = serde_json::from_str(&json)
                .map_err(|e| AppError::Internal(format!("Failed to deserialize user ID: {}", e)))?;
            Ok(Some(user_id))
        }
        None => Ok(None),
    }
}

/// 删除会话缓存
pub async fn delete_session(
    redis_client: &Client,
    session_id: &str,
) -> Result<(), AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let key = format!("session:{}", session_id);
    let _: RedisResult<i32> = conn.del(&key).await;

    Ok(())
}

/// Redis健康检查
pub async fn redis_health_check(redis_client: &Client) -> Result<bool, AppError> {
    let mut conn = redis_client
        .get_multiplexed_async_connection()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get Redis connection: {}", e)))?;

    let pong: String = cmd("PING")
        .query_async(&mut conn)
        .await
        .map_err(|e| AppError::Internal(format!("Redis health check failed: {}", e)))?;

    Ok(pong == "PONG")
}