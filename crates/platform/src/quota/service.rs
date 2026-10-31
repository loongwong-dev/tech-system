//! 配额摘要计算。

use chrono::{DateTime, Datelike, FixedOffset, TimeZone, Utc};
use db::AdminPool;
use uuid::Uuid;

use crate::quota::repository;

const FREE_MONTHLY_QUOTA: i64 = 15;

const QUOTA_METRIC: &str = "feedback_generated";
const UTC_OFFSET_SECONDS: i32 = 8 * 60 * 60;

/// 配额只读摘要。
#[derive(Debug, Clone)]
pub struct QuotaSummary {
    /// 本月总额度。
    pub limit: i64,
    /// 本月已用额度。
    pub used: i64,
    /// 剩余额度，最小为 0。
    pub remaining: i64,
    /// 下次重置时间。
    pub reset_at: DateTime<Utc>,
}

/// 配额摘要错误。
#[derive(Debug, thiserror::Error)]
pub enum QuotaError {
    /// 数据库操作失败。
    #[error("配额数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
    /// 固定时区配置无效。
    #[error("配额时区配置无效")]
    InvalidTimezone,
}

/// 计算租户当前自然月的配额摘要（东八区）。
///
/// # Errors
/// 数据库失败或时区配置异常时返回 [`QuotaError`]。
pub async fn quota_summary(pool: &AdminPool, tenant_id: Uuid) -> Result<QuotaSummary, QuotaError> {
    let now = Utc::now();
    let offset = FixedOffset::east_opt(UTC_OFFSET_SECONDS).ok_or(QuotaError::InvalidTimezone)?;
    let local_now = now.with_timezone(&offset);
    let period_start = offset
        .with_ymd_and_hms(local_now.year(), local_now.month(), 1, 0, 0, 0)
        .single()
        .ok_or(QuotaError::InvalidTimezone)?;
    let (next_year, next_month) = if local_now.month() == 12 {
        (local_now.year() + 1, 1)
    } else {
        (local_now.year(), local_now.month() + 1)
    };
    let reset_at = offset
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .ok_or(QuotaError::InvalidTimezone)?
        .with_timezone(&Utc);

    let used = repository::used(
        pool.pool(),
        tenant_id,
        period_start.date_naive(),
        QUOTA_METRIC,
    )
    .await
    .map_err(|source| QuotaError::Database { source })?;
    let delta = repository::entitlement_delta(pool.pool(), tenant_id, now)
        .await
        .map_err(|source| QuotaError::Database { source })?;
    let limit = (FREE_MONTHLY_QUOTA + delta).max(0);

    Ok(QuotaSummary {
        limit,
        used,
        remaining: (limit - used).max(0),
        reset_at,
    })
}
