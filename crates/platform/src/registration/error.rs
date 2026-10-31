//! 注册错误。

use crate::auth::PasswordError;
use crate::invite::InviteCodeError;

/// 注册流程错误。
#[derive(Debug, thiserror::Error)]
pub enum RegistrationError {
    /// 邮箱格式无效。
    #[error("邮箱格式无效")]
    InvalidEmail,
    /// 密码不符合长度或弱密码要求。
    #[error("密码不符合要求")]
    WeakPassword,
    /// 邀请码无效、过期或用尽。
    #[error("邀请码无效")]
    InvalidInviteCode,
    /// 邮箱已存在。
    #[error("邮箱已注册")]
    EmailExists,
    /// 密码哈希或内部处理失败。
    #[error("注册内部错误")]
    Internal,
    /// 数据库操作失败。
    #[error("注册数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}

impl From<InviteCodeError> for RegistrationError {
    fn from(error: InviteCodeError) -> Self {
        match error {
            InviteCodeError::Invalid | InviteCodeError::InvalidParameters => {
                Self::InvalidInviteCode
            }
            InviteCodeError::Database { source } => Self::Database { source },
        }
    }
}

impl From<PasswordError> for RegistrationError {
    fn from(error: PasswordError) -> Self {
        match error {
            PasswordError::TooShort | PasswordError::TooLong | PasswordError::Weak => {
                Self::WeakPassword
            }
            PasswordError::Hash => Self::Internal,
        }
    }
}
