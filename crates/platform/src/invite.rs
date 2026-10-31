//! 邀请码生成、规范化与原子消耗。
//!
//! 数据库只保存 SHA-256 哈希；明文只在生成时返回一次。

mod error;
mod model;
mod repository;
mod service;

pub use error::InviteCodeError;
pub use service::{
    GeneratedInviteCode, consume_invite_code, consume_invite_code_in_transaction,
    generate_invite_code,
};
