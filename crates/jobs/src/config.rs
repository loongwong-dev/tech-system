//! worker 运行参数。
//!
//! 配置从环境变量读取，测试可直接构造确定性的短间隔。

use std::time::Duration;

use crate::JobError;

/// worker 轮询、租约与回收参数。
#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// worker 标识，写入 `jobs.locked_by`。
    pub worker_id: String,
    /// 没有可领取任务时的等待时间。
    pub poll_interval: Duration,
    /// 领取后的租约长度；必须大于单个任务最长耗时。
    pub lease: Duration,
    /// 回收过期租约的扫描间隔。
    pub reclaim_interval: Duration,
}

impl WorkerConfig {
    /// 默认轮询间隔。
    pub const DEFAULT_POLL: Duration = Duration::from_secs(1);
    /// 默认租约；ADR-0002 约定 120 秒。
    pub const DEFAULT_LEASE: Duration = Duration::from_secs(120);
    /// 默认回收扫描间隔；ADR-0002 约定 30 秒。
    pub const DEFAULT_RECLAIM: Duration = Duration::from_secs(30);

    /// 构造配置；测试可传入毫秒级间隔。
    #[must_use]
    pub fn new(
        worker_id: impl Into<String>,
        poll_interval: Duration,
        lease: Duration,
        reclaim_interval: Duration,
    ) -> Self {
        Self {
            worker_id: worker_id.into(),
            poll_interval,
            lease,
            reclaim_interval,
        }
    }

    /// 从环境变量读取配置。
    ///
    /// 支持的变量：
    /// - `WORKER_ID`：默认 `worker-<pid>`；
    /// - `WORKER_POLL_SECONDS`：默认 1；
    /// - `WORKER_LEASE_SECONDS`：默认 120；
    /// - `WORKER_RECLAIM_SECONDS`：默认 30。
    ///
    /// # Errors
    /// 变量存在但不是正整数，或 `WORKER_ID` 为空时返回 [`JobError::Config`]。
    pub fn from_env() -> Result<Self, JobError> {
        let worker_id = match std::env::var("WORKER_ID") {
            Ok(value) if !value.trim().is_empty() => value,
            Ok(_) => {
                return Err(JobError::Config {
                    reason: "WORKER_ID 不能为空".to_owned(),
                });
            }
            Err(std::env::VarError::NotPresent) => format!("worker-{}", std::process::id()),
            Err(std::env::VarError::NotUnicode(_)) => {
                return Err(JobError::Config {
                    reason: "WORKER_ID 不是合法 UTF-8".to_owned(),
                });
            }
        };

        Ok(Self {
            worker_id,
            poll_interval: seconds_from_env("WORKER_POLL_SECONDS", Self::DEFAULT_POLL.as_secs())?,
            lease: seconds_from_env("WORKER_LEASE_SECONDS", Self::DEFAULT_LEASE.as_secs())?,
            reclaim_interval: seconds_from_env(
                "WORKER_RECLAIM_SECONDS",
                Self::DEFAULT_RECLAIM.as_secs(),
            )?,
        })
    }
}

fn seconds_from_env(name: &str, default: u64) -> Result<Duration, JobError> {
    match std::env::var(name) {
        Ok(value) => {
            let seconds = value.parse::<u64>().map_err(|_| JobError::Config {
                reason: format!("{name} 必须是正整数"),
            })?;
            if seconds == 0 {
                return Err(JobError::Config {
                    reason: format!("{name} 必须大于 0"),
                });
            }
            Ok(Duration::from_secs(seconds))
        }
        Err(std::env::VarError::NotPresent) => Ok(Duration::from_secs(default)),
        Err(std::env::VarError::NotUnicode(_)) => Err(JobError::Config {
            reason: format!("{name} 不是合法 UTF-8"),
        }),
    }
}
