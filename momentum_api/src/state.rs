//! Application state shared across all request handlers

use crate::routes::refresh_token_store::RefreshTokenStore;
use momentum_core::db::DbPool;
use momentum_core::services::jwt::JwtService;
use momentum_core::utils::AssetUrlHelper;
use redis::Client as RedisClient;
use std::sync::Arc;
use std::time::Duration;

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
    /// Refresh token 旋转存储（Issue #10）
    pub refresh_token_store: RefreshTokenStore,
    /// JWT signing service（Issue #10：startup 时构造一次，refresh 路由复用）
    pub jwt_service: Arc<JwtService>,
}

impl AppState {
    pub fn new(
        db: DbPool,
        redis: RedisClient,
        asset_helper: AssetUrlHelper,
        bcrypt_cost: u32,
        jwt_service: JwtService,
    ) -> Self {
        // Issue #10：默认 100k token 容量，7 天 TTL（与 refresh_token 有效期匹配）
        let refresh_token_store =
            RefreshTokenStore::new(100_000, Duration::from_secs(7 * 24 * 3600));
        Self {
            db,
            redis,
            asset_helper: Arc::new(asset_helper),
            bcrypt_cost,
            refresh_token_store,
            jwt_service: Arc::new(jwt_service),
        }
    }
}
