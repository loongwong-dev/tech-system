//! 配额只读摘要；扣减与权益 enforcement 由后续任务实现。

mod repository;
mod service;

pub use service::{QuotaError, QuotaSummary, quota_summary};
