//! 认证相关 SQL 访问。

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, Postgres, Transaction};
use uuid::Uuid;

/// 登录所需的用户字段。
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct UserRow {
    pub(crate) id: Uuid,
    pub(crate) password_hash: String,
    pub(crate) status: String,
}

/// 会话校验所需的字段。
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct SessionRow {
    pub(crate) id: Uuid,
    pub(crate) user_id: Uuid,
    pub(crate) tenant_id: Option<Uuid>,
    pub(crate) expires_at: DateTime<Utc>,
    pub(crate) revoked_at: Option<DateTime<Utc>>,
}

/// 按邮箱大小写不敏感查找用户。
pub(crate) async fn find_user<'e, E>(
    executor: E,
    email: &str,
) -> Result<Option<UserRow>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_as!(
        UserRow,
        r#"
        SELECT id, password_hash, status
        FROM users
        WHERE lower(email) = lower($1)
        LIMIT 1
        "#,
        email
    )
    .fetch_optional(executor)
    .await
}

/// 返回用户最早加入的租户。
pub(crate) async fn first_tenant<'e, E>(
    executor: E,
    user_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_scalar!(
        r#"
        SELECT tenant_id
        FROM memberships
        WHERE user_id = $1
        ORDER BY created_at
        LIMIT 1
        "#,
        user_id
    )
    .fetch_optional(executor)
    .await
}

/// 写入会话。
pub(crate) async fn insert_session(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    tenant_id: Option<Uuid>,
    token_hash: &[u8],
    expires_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO auth_sessions (user_id, tenant_id, token_hash, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        user_id,
        tenant_id,
        token_hash,
        expires_at
    )
    .fetch_one(transaction.as_mut())
    .await
}

/// 按 token 哈希吊销有效会话；零行表示不存在、已吊销或已过期。
pub(crate) async fn revoke_by_hash<'e, E>(
    executor: E,
    token_hash: &[u8],
) -> Result<bool, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_scalar!(
        r#"
        UPDATE auth_sessions
        SET revoked_at = now()
        WHERE token_hash = $1
          AND revoked_at IS NULL
          AND expires_at > now()
        RETURNING id
        "#,
        token_hash
    )
    .fetch_optional(executor)
    .await
    .map(|id| id.is_some())
}

/// 按 token 哈希读取会话。
pub(crate) async fn find_session<'e, E>(
    executor: E,
    token_hash: &[u8],
) -> Result<Option<SessionRow>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_as!(
        SessionRow,
        r#"
        SELECT id, user_id, tenant_id, expires_at, revoked_at
        FROM auth_sessions
        WHERE token_hash = $1
        "#,
        token_hash
    )
    .fetch_optional(executor)
    .await
}

/// 满足阈值条件时滑动续期。
pub(crate) async fn renew_session<'e, E>(
    executor: E,
    session_id: Uuid,
    expires_at: DateTime<Utc>,
    threshold: DateTime<Utc>,
) -> Result<(), sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query!(
        r#"
        UPDATE auth_sessions
        SET expires_at = $2
        WHERE id = $1
          AND revoked_at IS NULL
          AND expires_at < $3
        "#,
        session_id,
        expires_at,
        threshold
    )
    .execute(executor)
    .await?;
    Ok(())
}
