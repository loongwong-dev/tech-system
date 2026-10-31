//! 租户、账号、会话、权限、配额、审计与邀请码。
//!
//! 允许依赖 `db` 与 `shared`；不得依赖 `domain` 或 `api`。

mod auth;
mod invite;
mod registration;

pub use auth::{
    LoginError, LoginInput, LoginResult, SESSION_TTL_SECONDS, SessionContext, SessionError,
    SessionToken, login, revoke_session, validate_session,
};
pub use invite::{GeneratedInviteCode, InviteCodeError, consume_invite_code, generate_invite_code};
pub use registration::{RegisteredAccount, RegistrationError, RegistrationInput, register};
