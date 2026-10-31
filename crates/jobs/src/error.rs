//! 队列与 worker 错误。
//!
//! 只覆盖基础设施错误；业务 handler 的失败由 worker 转成 `last_error`。

/// 队列操作错误。
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    /// 入队参数非法。
    #[error("任务参数非法：{reason}")]
    InvalidInput {
        /// 面向人的原因。
        reason: &'static str,
    },

    /// worker 配置非法。
    #[error("worker 配置非法：{reason}")]
    Config {
        /// 面向人的原因。
        reason: String,
    },

    /// 数据库查询失败。
    #[error("任务队列数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
