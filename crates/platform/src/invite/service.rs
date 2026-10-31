//! 邀请码业务操作。

use chrono::{DateTime, Utc};
use db::AdminPool;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::invite::error::InviteCodeError;
use crate::invite::{model, repository};

/// 新生成的邀请码；明文只在此结构中出现一次。
pub struct GeneratedInviteCode {
    /// 邀请码主键。
    pub id: Uuid,
    /// 10 字符明文邀请码。
    pub code: String,
}

impl std::fmt::Debug for GeneratedInviteCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GeneratedInviteCode")
            .field("id", &self.id)
            .field("code", &"<redacted>")
            .finish()
    }
}

/// 生成邀请码，数据库只保存哈希。
///
/// # Errors
/// `max_uses` 小于等于 0 时返回 [`InviteCodeError::InvalidParameters`]；
/// 数据库失败时返回 [`InviteCodeError::Database`]。
pub async fn generate_invite_code(
    pool: &AdminPool,
    note: Option<&str>,
    max_uses: i32,
    expires_at: Option<DateTime<Utc>>,
) -> Result<GeneratedInviteCode, InviteCodeError> {
    if max_uses <= 0 {
        return Err(InviteCodeError::InvalidParameters);
    }

    let code = model::generate_code();
    let code_hash = model::hash_code(&code);
    let id = repository::insert(pool.pool(), &code_hash, note, max_uses, expires_at)
        .await
        .map_err(|source| InviteCodeError::Database { source })?;

    Ok(GeneratedInviteCode { id, code })
}

/// 原子消耗邀请码，返回邀请码主键。
///
/// 过期、用尽、不存在与格式错误统一返回 [`InviteCodeError::Invalid`]。
/// # Errors
/// 邀请码无效时返回 [`InviteCodeError::Invalid`]；
/// 数据库失败时返回 [`InviteCodeError::Database`]。
pub async fn consume_invite_code(pool: &AdminPool, code: &str) -> Result<Uuid, InviteCodeError> {
    let mut transaction = pool
        .pool()
        .begin()
        .await
        .map_err(|source| InviteCodeError::Database { source })?;
    let id = consume_invite_code_in_transaction(&mut transaction, code).await?;
    transaction
        .commit()
        .await
        .map_err(|source| InviteCodeError::Database { source })?;
    Ok(id)
}

/// 在调用方事务内消耗邀请码。
///
/// 注册流程依赖该入口把邀请码消耗与用户、租户、会话写入放在同一事务。
/// # Errors
/// 邀请码无效时返回 [`InviteCodeError::Invalid`]；
/// 数据库失败时返回 [`InviteCodeError::Database`]。
pub async fn consume_invite_code_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    code: &str,
) -> Result<Uuid, InviteCodeError> {
    let normalized = model::normalize_code(code).ok_or(InviteCodeError::Invalid)?;
    let code_hash = model::hash_code(&normalized);

    repository::consume(transaction.as_mut(), &code_hash)
        .await
        .map_err(|source| InviteCodeError::Database { source })?
        .ok_or(InviteCodeError::Invalid)
}
