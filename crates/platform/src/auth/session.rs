//! 登录会话 token 生成与持久化。

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{TimeDelta, Utc};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// 会话有效期：30 天。
pub const SESSION_TTL_SECONDS: i64 = 30 * 24 * 60 * 60;

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

/// 创建会话并写入 `auth_sessions`。
pub(crate) async fn create_session(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tenant_id: Uuid,
) -> Result<SessionToken, sqlx::Error> {
    let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
    let token_hash = Sha256::digest(token.as_bytes());
    let expires_at = Utc::now() + TimeDelta::seconds(SESSION_TTL_SECONDS);

    sqlx::query!(
        r#"
        INSERT INTO auth_sessions (user_id, tenant_id, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id,
        tenant_id,
        token_hash.as_slice(),
        expires_at
    )
    .execute(transaction.as_mut())
    .await?;

    Ok(SessionToken(token))
}
