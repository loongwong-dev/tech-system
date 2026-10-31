//! 密码哈希与登录会话。

mod login;
mod me;
mod password;
mod repository;
mod session;
mod switch;
mod tenants;

pub use login::{LoginError, LoginInput, LoginResult, login};
pub use me::{MeError, MeView, me};
pub(crate) use password::{PasswordError, hash_password, validate_password};
pub(crate) use session::create_session;
pub use session::{
    SESSION_TTL_SECONDS, SessionContext, SessionError, SessionToken, revoke_session,
    validate_session,
};
pub use switch::{SwitchTenantError, switch_tenant};
pub use tenants::{TenantListError, TenantSummary, list_tenants};
