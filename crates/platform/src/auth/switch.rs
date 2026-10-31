//! 租户切换。

use db::AdminPool;
use uuid::Uuid;

use crate::auth::repository;
use crate::auth::session::{
    SessionError, SessionToken, create_session, hash_token, validate_session,
};

/// 租户切换错误。
#[derive(Debug, thiserror::Error)]
pub enum SwitchTenantError {
    /// 用户不属于目标租户。
    #[error("无权切换到该租户")]
    ForbiddenTenant,
    /// 当前会话已失效。
    #[error("会话已失效")]
    SessionExpired,
    /// 数据库操作失败。
    #[error("租户切换数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}

/// 校验 membership 后吊销旧会话并创建指向新租户的会话。
///
/// # Errors
/// 非成员返回 [`SwitchTenantError::ForbiddenTenant`]；
/// 会话失效或数据库失败时返回对应错误。
pub async fn switch_tenant(
    pool: &AdminPool,
    token: &str,
    tenant_id: Uuid,
) -> Result<SessionToken, SwitchTenantError> {
    let context = validate_session(pool, token)
        .await
        .map_err(map_session_error)?;

    let mut transaction = pool
        .pool()
        .begin()
        .await
        .map_err(|source| SwitchTenantError::Database { source })?;

    let role = repository::membership_role(transaction.as_mut(), tenant_id, context.user_id)
        .await
        .map_err(|source| SwitchTenantError::Database { source })?;
    if role.is_none() {
        return Err(SwitchTenantError::ForbiddenTenant);
    }

    let revoked = repository::revoke_by_hash(transaction.as_mut(), &hash_token(token))
        .await
        .map_err(|source| SwitchTenantError::Database { source })?;
    if !revoked {
        return Err(SwitchTenantError::SessionExpired);
    }

    let session_token = create_session(&mut transaction, context.user_id, Some(tenant_id))
        .await
        .map_err(|source| SwitchTenantError::Database { source })?;
    transaction
        .commit()
        .await
        .map_err(|source| SwitchTenantError::Database { source })?;

    Ok(session_token)
}

fn map_session_error(error: SessionError) -> SwitchTenantError {
    match error {
        SessionError::Expired => SwitchTenantError::SessionExpired,
        SessionError::Database { source } => SwitchTenantError::Database { source },
    }
}
