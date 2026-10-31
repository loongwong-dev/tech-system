//! 任务模型：队列写入参数与查询结果。
//!
//! 任务属于租户；payload 只放业务输入，不放凭据。

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

/// 等待领取。
pub const STATUS_PENDING: &str = "pending";
/// 已被 worker 领取并持有租约。
pub const STATUS_RUNNING: &str = "running";
/// 执行成功。
pub const STATUS_SUCCEEDED: &str = "succeeded";
/// 永久失败，进入死信。
pub const STATUS_DEAD: &str = "dead";
/// 被调用方取消。
pub const STATUS_CANCELLED: &str = "cancelled";

/// 一条后台任务。
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Job {
    /// 任务主键。
    pub id: Uuid,
    /// 所属租户；执行时必须重新开启租户事务。
    pub tenant_id: Uuid,
    /// 任务类型。
    pub kind: String,
    /// 活跃任务幂等键。
    pub dedupe_key: Option<String>,
    /// 任务输入。
    pub payload: Value,
    /// 当前状态。
    pub status: String,
    /// 已领取次数。
    pub attempts: i32,
    /// 最大领取次数。
    pub max_attempts: i32,
    /// 最早可领取时间。
    pub run_at: DateTime<Utc>,
    /// 当前 worker 标识。
    pub locked_by: Option<String>,
    /// 租约到期时间。
    pub locked_until: Option<DateTime<Utc>>,
    /// 最近一次失败原因。
    pub last_error: Option<String>,
    /// 入队时间。
    pub created_at: DateTime<Utc>,
    /// 最后变更时间。
    pub updated_at: DateTime<Utc>,
    /// 终态时间。
    pub finished_at: Option<DateTime<Utc>>,
}

/// 新任务参数。
#[derive(Debug, Clone)]
pub struct NewJob {
    /// 所属租户。
    pub tenant_id: Uuid,
    /// 任务类型。
    pub kind: String,
    /// 可选幂等键；同一租户、类型与键在活跃状态下只保留一条。
    pub dedupe_key: Option<String>,
    /// 任务输入。
    pub payload: Value,
    /// 最大领取次数。
    pub max_attempts: i32,
    /// 最早可领取时间。
    pub run_at: DateTime<Utc>,
}

impl NewJob {
    /// 构造默认任务：最多 5 次尝试，立即可领取。
    #[must_use]
    pub fn new(tenant_id: Uuid, kind: impl Into<String>, payload: Value) -> Self {
        Self {
            tenant_id,
            kind: kind.into(),
            dedupe_key: None,
            payload,
            max_attempts: 5,
            run_at: Utc::now(),
        }
    }

    /// 设置幂等键。
    #[must_use]
    pub fn with_dedupe_key(mut self, key: impl Into<String>) -> Self {
        self.dedupe_key = Some(key.into());
        self
    }

    /// 设置最大领取次数。
    #[must_use]
    pub fn with_max_attempts(mut self, max_attempts: i32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    /// 设置最早可领取时间。
    #[must_use]
    pub fn with_run_at(mut self, run_at: DateTime<Utc>) -> Self {
        self.run_at = run_at;
        self
    }
}
