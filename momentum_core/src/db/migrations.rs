//! Embedded database migrations
//!
//! 使用 diesel_migrations 的 embedded 特性，在应用启动时自动运行待执行的迁移。
//! 这确保新部署的实例无需手动运行 `diesel migration run`。

use diesel::PgConnection;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness};
use std::error::Error;

/// 嵌入的迁移文件（从 ./migrations 目录）
/// Diesel 会自动查找并编译这些迁移
pub const MIGRATIONS: EmbeddedMigrations = diesel_migrations::embed_migrations!("migrations");

/// 运行所有待执行的迁移
///
/// # 错误处理
///
/// - 如果迁移运行失败，返回具体错误信息
/// - 已运行的迁移会被跳过（diesel_migrations 自动处理）
///
/// # 使用示例
///
/// ```rust,ignore
/// use diesel::r2d2::Pool;
/// use momentum_core::db::run_pending_migrations;
///
/// let pool = Pool::builder().build(connection_manager)?;
/// run_pending_migrations(&pool)?;
/// ```
pub fn run_pending_migrations(
    pool: &diesel::r2d2::Pool<diesel::r2d2::ConnectionManager<PgConnection>>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut conn = pool.get().map_err(|e| format!("Failed to get connection: {}", e))?;

    // 在新线程中运行迁移，因为 diesel_migrations 是同步的
    let handle = std::thread::spawn(move || {
        run_migrations_internal(&mut conn)
    });

    handle.join().map_err(|e| format!("Migration thread panicked: {:?}", e))?
}

fn run_migrations_internal(conn: &mut PgConnection) -> Result<(), Box<dyn Error + Send + Sync>> {
    // 执行待运行的迁移
    conn.run_pending_migrations(MIGRATIONS)
        .map_err(|e| format!("Failed to run migrations: {}", e))?;

    tracing::info!("Database migrations completed successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_migrations_embed() {
        // 验证 MIGRATIONS 常量存在且可访问
        use super::MIGRATIONS;
        let _ = MIGRATIONS;
    }
}
