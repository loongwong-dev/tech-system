use sqlx::PgPool;

use crate::DbError;

/// 检查单个连接池是否可用。
///
/// 只执行常量查询，不访问业务表，避免健康检查被迁移状态影响。
pub(crate) async fn ping(pool: &PgPool, name: &'static str) -> Result<(), DbError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(pool)
        .await
        .map(|_| ())
        .map_err(|source| DbError::Health { pool: name, source })
}
