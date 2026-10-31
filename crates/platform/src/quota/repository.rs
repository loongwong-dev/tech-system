//! 配额 SQL 访问。

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

/// 读取本月已用额度。
pub(crate) async fn used<'e, E>(
    executor: E,
    tenant_id: Uuid,
    period: NaiveDate,
    metric: &str,
) -> Result<i64, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let used: Option<i32> = sqlx::query_scalar!(
        r#"
        SELECT used
        FROM usage_counters
        WHERE tenant_id = $1 AND period = $2 AND metric = $3
        "#,
        tenant_id,
        period,
        metric
    )
    .fetch_optional(executor)
    .await?;
    Ok(i64::from(used.unwrap_or(0)))
}

/// 汇总当前有效的增值包额度。
pub(crate) async fn entitlement_delta<'e, E>(
    executor: E,
    tenant_id: Uuid,
    now: DateTime<Utc>,
) -> Result<i64, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let delta: Option<i64> = sqlx::query_scalar!(
        r#"
        SELECT COALESCE(SUM(quota_delta), 0)::bigint
        FROM entitlements
        WHERE tenant_id = $1
          AND starts_at <= $2
          AND (ends_at IS NULL OR ends_at > $2)
        "#,
        tenant_id,
        now
    )
    .fetch_one(executor)
    .await?;
    Ok(delta.unwrap_or(0))
}
