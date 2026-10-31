//! 邀请码错误。

/// 邀请码操作错误。
#[derive(Debug, thiserror::Error)]
pub enum InviteCodeError {
    /// 邀请码无效、过期、用尽或不存在；对外统一返回该错误。
    #[error("邀请码无效")]
    Invalid,

    /// 生成参数不合法，例如 `max_uses` 小于等于 0。
    #[error("邀请码参数不合法")]
    InvalidParameters,

    /// 数据库操作失败。
    #[error("邀请码数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
