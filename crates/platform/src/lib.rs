//! 租户、账号、会话、权限、配额、审计与邀请码。
//!
//! 允许依赖 `db` 与 `shared`；不得依赖 `domain` 或 `api`。

mod auth;
mod invite;
mod quota;
mod registration;

pub use auth::{
    LoginError, LoginInput, LoginResult, MeError, MeView, SESSION_TTL_SECONDS, SessionContext,
    SessionError, SessionToken, SwitchTenantError, login, me, revoke_session, switch_tenant,
    validate_session,
};
pub use invite::{GeneratedInviteCode, InviteCodeError, consume_invite_code, generate_invite_code};
pub use quota::{QuotaError, QuotaSummary, quota_summary};
pub use registration::{RegisteredAccount, RegistrationError, RegistrationInput, register};
