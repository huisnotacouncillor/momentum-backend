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
}

impl AppState {
    pub fn new(db: DbPool, redis: RedisClient, asset_helper: AssetUrlHelper) -> Self {
        Self {
            db,
            redis,
            asset_helper: Arc::new(asset_helper),
        }
    }
}
