//! 当前用户可访问的租户列表。

use db::AdminPool;
use uuid::Uuid;

use crate::auth::repository;

/// 会话用户可切换的租户。
#[derive(Debug, Clone)]
pub struct TenantSummary {
    /// 租户主键。
    pub id: Uuid,
    /// 租户名称。
    pub name: String,
    /// 用户在该租户的角色。
    pub role: String,
}

/// 租户列表查询错误。
#[derive(Debug, thiserror::Error)]
pub enum TenantListError {
    /// 数据库操作失败。
    #[error("读取租户列表失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}

/// 列出用户仍然是成员且状态为 active 的租户。
///
/// # Errors
/// 数据库查询失败时返回 [`TenantListError::Database`]。
pub async fn list_tenants(
    pool: &AdminPool,
    user_id: Uuid,
) -> Result<Vec<TenantSummary>, TenantListError> {
    let rows = repository::list_user_tenants(pool.pool(), user_id)
        .await
        .map_err(|source| TenantListError::Database { source })?;
    Ok(rows
        .into_iter()
        .map(|row| TenantSummary {
            id: row.id,
            name: row.name,
            role: row.role,
        })
        .collect())
}
