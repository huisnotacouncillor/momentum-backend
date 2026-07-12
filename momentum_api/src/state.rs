//! Application state shared across all request handlers

use momentum_core::db::DbPool;
use momentum_core::utils::AssetUrlHelper;
use redis::Client as RedisClient;
use std::sync::Arc;

/// Application state shared across all request handlers
#[derive(Clone)]
pub struct AppState {
    /// Database connection pool
    pub db: DbPool,
    /// Redis client for caching
    pub redis: RedisClient,
    /// Asset URL helper
    pub asset_helper: Arc<AssetUrlHelper>,
    /// bcrypt cost（从 Config.bcrypt_cost 传入）。Issue #9：register 路由显式传
    /// 此值，不再硬编码 bcrypt::DEFAULT_COST。
    pub bcrypt_cost: u32,
}

impl AppState {
    pub fn new(db: DbPool, redis: RedisClient, asset_helper: AssetUrlHelper, bcrypt_cost: u32) -> Self {
        Self {
            db,
            redis,
            asset_helper: Arc::new(asset_helper),
            bcrypt_cost,
        }
    }
}
