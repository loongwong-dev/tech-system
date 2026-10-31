//! 邀请码 SQL 访问。

use chrono::{DateTime, Utc};
use sqlx::{PgExecutor, PgPool};
use uuid::Uuid;

/// 插入邀请码，返回主键。
pub(crate) async fn insert(
    pool: &PgPool,
    code_hash: &[u8],
    note: Option<&str>,
    max_uses: i32,
    expires_at: Option<DateTime<Utc>>,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO invite_codes (code_hash, note, max_uses, expires_at)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
        code_hash,
        note,
        max_uses,
        expires_at
    )
    .fetch_one(pool)
    .await
}

/// 原子消耗一次邀请码，返回邀请码主键；零行表示无效。
///
/// 接受连接池或事务连接，便于注册流程与用户/租户写入共享同一事务。
pub(crate) async fn consume<'e, E>(
    executor: E,
    code_hash: &[u8],
) -> Result<Option<Uuid>, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    sqlx::query_scalar!(
        r#"
        UPDATE invite_codes
        SET used_count = used_count + 1
        WHERE code_hash = $1
          AND used_count < max_uses
          AND (expires_at IS NULL OR expires_at > now())
        RETURNING id
        "#,
        code_hash
    )
    .fetch_optional(executor)
    .await
}
