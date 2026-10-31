//! 当前用户与租户摘要。

use db::AdminPool;
use uuid::Uuid;

use crate::auth::repository;
use crate::quota::{QuotaError, QuotaSummary, quota_summary};

/// `/api/v1/me` 的领域视图。
#[derive(Debug)]
pub struct MeView {
    /// 当前用户主键。
    pub user_id: Uuid,
    /// 当前用户邮箱。
    pub email: String,
    /// 当前租户主键。
    pub tenant_id: Uuid,
    /// 当前租户名称。
    pub tenant_name: String,
    /// 当前租户内角色。
    pub role: String,
    /// 本月配额摘要。
    pub quota: QuotaSummary,
}

/// `/me` 查询错误。
#[derive(Debug, thiserror::Error)]
pub enum MeError {
    /// 用户、租户或成员关系不存在。
    #[error("账号或租户不存在")]
    NotFound,
    /// 数据库操作失败。
    #[error("读取账号信息失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
    /// 配额摘要计算失败。
    #[error("读取配额失败")]
    Quota {
        /// 底层配额错误。
        #[source]
        source: QuotaError,
    },
}

/// 读取当前用户、租户、角色与配额。
///
/// # Errors
/// 数据缺失或数据库失败时返回 [`MeError`]。
pub async fn me(pool: &AdminPool, user_id: Uuid, tenant_id: Uuid) -> Result<MeView, MeError> {
    let email = repository::user_email(pool.pool(), user_id)
        .await
        .map_err(|source| MeError::Database { source })?
        .ok_or(MeError::NotFound)?;
    let tenant_name = repository::tenant_name(pool.pool(), tenant_id)
        .await
        .map_err(|source| MeError::Database { source })?
        .ok_or(MeError::NotFound)?;
    let role = repository::membership_role(pool.pool(), tenant_id, user_id)
        .await
        .map_err(|source| MeError::Database { source })?
        .ok_or(MeError::NotFound)?;
    let quota = quota_summary(pool, tenant_id)
        .await
        .map_err(|source| MeError::Quota { source })?;

    Ok(MeView {
        user_id,
        email,
        tenant_id,
        tenant_name,
        role,
        quota,
    })
}
