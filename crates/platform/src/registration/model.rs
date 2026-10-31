//! 注册输入与结果。

use uuid::Uuid;

use crate::auth::SessionToken;

/// 注册输入。
pub struct RegistrationInput {
    /// 登录邮箱。
    pub email: String,
    /// 明文密码，只用于本次哈希。
    pub password: String,
    /// 明文邀请码，只用于本次消耗。
    pub invite_code: String,
}

/// 注册成功后的账号信息。
#[derive(Debug)]
pub struct RegisteredAccount {
    /// 新用户主键。
    pub user_id: Uuid,
    /// 新租户主键。
    pub tenant_id: Uuid,
    /// 会话 token；由 API 写入 `sid` Cookie。
    pub session_token: SessionToken,
}
