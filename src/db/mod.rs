pub mod enums;
pub mod models;
pub mod repositories;

use diesel::PgConnection;
use diesel::r2d2::{self, ConnectionManager as DbConnectionManager};
use crate::config::DatabaseConfig;
use crate::error::{AppError, AppResult};

pub type DbPool = r2d2::Pool<DbConnectionManager<PgConnection>>;

pub fn create_pool(config: &DatabaseConfig) -> AppResult<DbPool> {
    let manager = DbConnectionManager::<PgConnection>::new(&config.url);

    r2d2::Pool::builder()
        .max_size(config.max_connections)
        .min_idle(Some(config.min_connections))
        .connection_timeout(std::time::Duration::from_secs(config.connection_timeout))
        .build(manager)
        .map_err(AppError::Pool)
}

pub async fn pool_health_check(pool: &DbPool) -> AppResult<()> {
    let state = pool.state();
    tracing::info!(
        connections = state.connections,
        idle_connections = state.idle_connections,
        "Database pool status"
    );

    // 测试连接
    let _conn = pool.get()?;
    Ok(())
}
