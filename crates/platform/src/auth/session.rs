//! 登录会话 token、滑动续期与吊销。

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{TimeDelta, Utc};
use db::AdminPool;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::repository;

/// 会话有效期：30 天。
pub const SESSION_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

const REFRESH_INTERVAL_SECONDS: i64 = 60 * 60;

/// 会话 token；只在创建时返回明文，数据库只存哈希。
pub struct SessionToken(String);

impl SessionToken {
    /// 返回明文 token，用于写入 Cookie。
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Debug for SessionToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("SessionToken(<redacted>)")
    }
}

/// 会话校验通过后的上下文。
#[derive(Debug)]
pub struct SessionContext {
    /// 当前用户。
    pub user_id: Uuid,
    /// 当前租户；用户尚未加入租户时为空。
    pub tenant_id: Option<Uuid>,
}

/// 会话校验或吊销错误。
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// 会话不存在、已过期或已吊销。
    #[error("会话已失效")]
    Expired,
    /// 数据库操作失败。
    #[error("会话数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}

/// 计算会话 token 的 SHA-256 哈希。
pub(crate) fn hash_token(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

/// 创建会话并写入 `auth_sessions`。
pub(crate) async fn create_session(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
) -> Result<SessionToken, sqlx::Error> {
    let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
    let token_hash = hash_token(&token);
    let expires_at = Utc::now() + TimeDelta::seconds(SESSION_TTL_SECONDS);

    repository::insert_session(transaction, user_id, tenant_id, &token_hash, expires_at).await?;

    Ok(SessionToken(token))
}

/// 校验会话并按小时最多续期一次。
///
/// # Errors
/// 会话不存在、已过期或已吊销时返回 [`SessionError::Expired`]。
pub async fn validate_session(
    pool: &AdminPool,
    token: &str,
) -> Result<SessionContext, SessionError> {
    let now = Utc::now();
    let session = repository::find_session(pool.pool(), &hash_token(token))
        .await
        .map_err(|source| SessionError::Database { source })?
        .ok_or(SessionError::Expired)?;

    if session.revoked_at.is_some() || session.expires_at <= now {
        return Err(SessionError::Expired);
    }

    let threshold = now + TimeDelta::seconds(SESSION_TTL_SECONDS - REFRESH_INTERVAL_SECONDS);
    if session.expires_at < threshold {
        repository::renew_session(
            pool.pool(),
            session.id,
            now + TimeDelta::seconds(SESSION_TTL_SECONDS),
            threshold,
        )
        .await
        .map_err(|source| SessionError::Database { source })?;
    }

    Ok(SessionContext {
        user_id: session.user_id,
        tenant_id: session.tenant_id,
    })
}

/// 吊销会话；已吊销、过期或不存在时返回 [`SessionError::Expired`]。
///
/// # Errors
/// 会话不存在、已过期或已吊销时返回 [`SessionError::Expired`]；
/// 数据库失败时返回 [`SessionError::Database`]。
pub async fn revoke_session(pool: &AdminPool, token: &str) -> Result<(), SessionError> {
    let revoked = repository::revoke_by_hash(pool.pool(), &hash_token(token))
        .await
        .map_err(|source| SessionError::Database { source })?;
    if revoked {
        Ok(())
    } else {
        Err(SessionError::Expired)
    }
}
