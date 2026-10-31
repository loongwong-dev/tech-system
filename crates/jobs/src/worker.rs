//! worker 循环与任务处理器注册表。
//!
//! worker 用管理池跨租户领取任务；handler 写入业务数据时必须自行开启
//! [`db::TenantPool::begin`]，不能复用领取连接或管理池。

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use sqlx::PgPool;
use tokio::sync::watch;

use crate::config::WorkerConfig;
use crate::error::JobError;
use crate::model::Job;
use crate::repository::{claim, complete, fail, reclaim_expired};

/// 异步任务处理器。
///
/// 返回 `Err` 时由队列记录失败并退避重试；handler 必须幂等，因为至少一次
/// 投递可能重复执行。
pub type Handler = Arc<
    dyn for<'a> Fn(&'a Job) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
        + Send
        + Sync,
>;

/// 任务类型到处理器的映射。
#[derive(Clone)]
pub struct Handlers {
    inner: Arc<HashMap<String, Handler>>,
}

impl Default for Handlers {
    fn default() -> Self {
        Self::new()
    }
}

impl Handlers {
    /// 创建空注册表。
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(HashMap::new()),
        }
    }

    /// 注册一个任务处理器；同类型重复注册会覆盖旧处理器。
    pub fn register<F>(&mut self, kind: impl Into<String>, handler: F) -> &mut Self
    where
        F: for<'a> Fn(&'a Job) -> Pin<Box<dyn Future<Output = Result<(), String>> + Send + 'a>>
            + Send
            + Sync
            + 'static,
    {
        let mut handlers = (*self.inner).clone();
        handlers.insert(kind.into(), Arc::new(handler));
        self.inner = Arc::new(handlers);
        self
    }

    fn get(&self, kind: &str) -> Option<Handler> {
        self.inner.get(kind).cloned()
    }

    /// 是否没有注册任何处理器。
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// 轮询 PostgreSQL 队列的 worker。
pub struct Worker {
    pool: PgPool,
    config: WorkerConfig,
    handlers: Handlers,
}

impl Worker {
    /// 构造 worker。
    ///
    /// handler 需要写业务数据时，必须自行持有租户池并按任务的 `tenant_id`
    /// 开启租户事务；不能复用领取连接或管理池。
    #[must_use]
    pub fn new(pool: PgPool, config: WorkerConfig, handlers: Handlers) -> Self {
        Self {
            pool,
            config,
            handlers,
        }
    }

    /// 运行 worker，直到收到关闭信号。
    ///
    /// 收到信号后停止领取新任务，等待当前 handler 完成，再返回。正在执行的
    /// 任务不会因为进程退出而丢失：若进程被强制杀死，租约到期后由回收循环
    /// 重新入队。
    ///
    /// # Errors
    /// 数据库操作失败时返回 [`JobError::Database`]；配置外的临时错误直接
    /// 暴露给进程入口，由进程管理器重启。
    pub async fn run(self, mut shutdown: watch::Receiver<bool>) -> Result<(), JobError> {
        let mut last_reclaim: Option<Instant> = None;

        loop {
            if *shutdown.borrow() {
                tracing::info!(worker_id = %self.config.worker_id, "worker 停止领取");
                break;
            }

            if last_reclaim
                .is_none_or(|last: Instant| last.elapsed() >= self.config.reclaim_interval)
            {
                let reclaimed = reclaim_expired(&self.pool, 100).await?;
                if reclaimed > 0 {
                    tracing::info!(count = reclaimed, "回收过期任务");
                }
                last_reclaim = Some(Instant::now());
            }

            if let Some(job) = claim(&self.pool, &self.config.worker_id, self.config.lease).await? {
                self.handle(job).await?;
                continue;
            }

            tokio::select! {
                () = tokio::time::sleep(self.config.poll_interval) => {}
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle(&self, job: Job) -> Result<(), JobError> {
        tracing::info!(
            job_id = %job.id,
            tenant_id = %job.tenant_id,
            kind = %job.kind,
            attempts = job.attempts,
            "领取任务"
        );

        let result = match self.handlers.get(&job.kind) {
            Some(handler) => handler(&job).await,
            None => Err(format!("未注册的任务类型：{}", job.kind)),
        };

        match result {
            Ok(()) => {
                if complete(&self.pool, job.id, &self.config.worker_id).await? {
                    tracing::info!(job_id = %job.id, "任务完成");
                } else {
                    tracing::warn!(job_id = %job.id, "任务完成写回被忽略：租约已失效或任务已取消");
                }
            }
            Err(error) => {
                if fail(&self.pool, job.id, &self.config.worker_id, &error).await? {
                    tracing::warn!(job_id = %job.id, error = %error, "任务失败");
                } else {
                    tracing::warn!(job_id = %job.id, "任务失败写回被忽略：租约已失效或任务已取消");
                }
            }
        }

        Ok(())
    }
}
