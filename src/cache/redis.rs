use redis::AsyncCommands;
use serde::{Serialize, de::DeserializeOwned};

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
