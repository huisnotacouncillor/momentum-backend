use redis::AsyncCommands;
use serde::{Serialize, de::DeserializeOwned};
use uuid::Uuid;
use crate::db::{models::auth::User, DbPool};
use diesel::prelude::*;

pub async fn get_cache<T: DeserializeOwned>(client: &redis::Client, key: &str) -> Option<T> {
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let value: String = conn.get(key).await.ok()?;
    serde_json::from_str(&value).ok()
}

pub async fn set_cache<T: Serialize>(client: &redis::Client, key: &str, value: &T, ttl: u64) {
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    let json = serde_json::to_string(value).unwrap();
    let _: () = conn.set_ex(key, json, ttl).await.unwrap();
}

pub async fn get_user_current_workspace_id(client: &redis::Client, user_id: Uuid) -> Option<Uuid> {
    let key = format!("user:{}:current_workspace_id", user_id);
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;
    let value: String = conn.get(&key).await.ok()?;
    value.parse::<Uuid>().ok()
}

pub async fn set_user_current_workspace_id(client: &redis::Client, user_id: Uuid, workspace_id: Uuid) -> Result<(), redis::RedisError> {
    let key = format!("user:{}:current_workspace_id", user_id);
    let mut conn = client.get_multiplexed_async_connection().await?;
    let _: () = conn.set_ex(&key, workspace_id.to_string(), 3600).await?; // 缓存1小时
    Ok(())
}

pub async fn get_user_current_workspace_id_cached(client: &redis::Client, pool: &DbPool, user_id: Uuid) -> Option<Uuid> {
    // 先尝试从Redis获取
    if let Some(workspace_id) = get_user_current_workspace_id(client, user_id).await {
        return Some(workspace_id);
    }
    
    // 如果Redis中没有，则从数据库获取
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(_) => return None,
    };
    
    let user = match crate::schema::users::table
        .filter(crate::schema::users::id.eq(user_id))
        .select(User::as_select())
        .first::<User>(&mut conn)
        .optional() {
            Ok(Some(user)) => user,
            _ => return None,
        };
        
    // 将从数据库获取的workspace_id存入Redis缓存
    if let Some(workspace_id) = user.current_workspace_id {
        let _ = set_user_current_workspace_id(client, user_id, workspace_id).await;
        Some(workspace_id)
    } else {
        None
    }
}