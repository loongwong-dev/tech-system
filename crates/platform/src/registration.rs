//! 邀请码注册：创建用户、租户、owner 关系与会话。

mod error;
mod model;
mod repository;
mod service;

pub use error::RegistrationError;
pub use model::{RegisteredAccount, RegistrationInput};
pub use service::register;
