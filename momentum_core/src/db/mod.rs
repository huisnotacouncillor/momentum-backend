pub mod enums;
pub mod models;
pub mod repositories;

use crate::config::DatabaseConfig;
use crate::error::{AppError, AppResult};
use diesel::PgConnection;
use diesel::r2d2::{self, ConnectionManager as DbConnectionManager};

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

/// P1.3 修复：使用 spawn_blocking 包装同步 DB 操作，避免阻塞 tokio 工作线程
///
/// Diesel 是同步 ORM，如果直接在 async 上下文中执行会阻塞 tokio 调度器，
/// 在高并发下会导致线程饥饿。该函数将 DB 操作 offload 到阻塞线程池。
///
/// # 使用示例
///
/// ```rust,ignore
/// use momentum_core::db::run_db;
///
/// // 在 async handler 中：
/// let issue = run_db(&state.db, move |conn| {
///     IssueRepo::find_by_id_in_workspace(conn, workspace_id, issue_id)
/// }).await?;
/// ```
///
/// # 参数
/// - `pool`: 数据库连接池引用
/// - `f`: 在同步上下文中执行的闭包，接收 `&mut PgConnection`
///
/// # 错误
/// - 连接池耗尽 → `AppError::ServiceUnavailable`
/// - 任务执行错误 → `AppError::Internal`
/// - 闭包返回的错误透传
pub async fn run_db<F, R>(pool: &DbPool, f: F) -> AppResult<R>
where
    F: FnOnce(&mut PgConnection) -> AppResult<R> + Send + 'static,
    R: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|_| AppError::ServiceUnavailable {
            message: "Database temporarily unavailable".to_string(),
        })?;
        f(&mut conn)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?
}

/// run_db 的 diesel::result::Error 便利版本
///
/// 用于那些返回 `Result<T, diesel::result::Error>` 的仓储函数
pub async fn run_db_diesel<F, R>(pool: &DbPool, f: F) -> AppResult<R>
where
    F: FnOnce(&mut PgConnection) -> Result<R, diesel::result::Error> + Send + 'static,
    R: Send + 'static,
{
    let pool = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|_| AppError::ServiceUnavailable {
            message: "Database temporarily unavailable".to_string(),
        })?;
        f(&mut conn).map_err(AppError::Database)
    })
    .await
    .map_err(|e| AppError::Internal(format!("Task join error: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_pool_type() {
        // 类型编译测试
        let _: Option<DbPool> = None;
    }
}
