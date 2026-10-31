//! 数据库连接池、迁移与仓储基元。
//!
//! 只允许依赖 `shared`；不得依赖 `domain`、`platform` 或 `api`。
//!
//! 双连接池按角色分离：租户池只能通过 [`TenantPool::begin`] 开启带
//! `app.tenant_id` 的事务，管理池用于跨租户的例外表访问。

mod config;
mod error;
mod health;
mod pool;

pub use config::DbConfig;
pub use error::DbError;
pub use pool::{AdminPool, Db, TenantPool, TenantTransaction};

use sqlx::PgPool;
use sqlx::migrate::Migrator;

/// 编译进二进制的数据库迁移集合。
///
/// 默认读取 crate 根目录下的 `migrations/`，避免跨 crate 的相对路径在编辑器
/// 与不同构建环境中解析失败。
static MIGRATOR: Migrator = sqlx::migrate!();

/// 在指定连接池上执行全部迁移。
///
/// 可重复调用：`sqlx` 通过 `_sqlx_migrations` 跳过已应用版本。
///
/// # Errors
/// 迁移失败时返回 [`DbError::Migrate`]。
pub async fn run_migrations(pool: &PgPool) -> Result<(), DbError> {
    MIGRATOR
        .run(pool)
        .await
        .map_err(|source| DbError::Migrate { source })
}
