//! PostgreSQL 任务队列、worker 与定时任务。
//!
//! 允许依赖 `domain`、`db` 与 `shared`；不得依赖 `api`。当前提供事务内
//! 入队、`SKIP LOCKED` 领取、租约回收、退避重试与优雅退出；业务 handler
//! 由调用方注册。

mod config;
mod error;
mod model;
mod repository;
mod worker;

pub use config::WorkerConfig;
pub use error::JobError;
pub use model::{
    Job, NewJob, STATUS_CANCELLED, STATUS_DEAD, STATUS_PENDING, STATUS_RUNNING, STATUS_SUCCEEDED,
};
pub use repository::{cancel, claim, complete, enqueue, fail, get, reclaim_expired};
pub use worker::{Handler, Handlers, Worker};
