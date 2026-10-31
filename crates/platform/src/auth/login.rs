//! 邮箱密码登录。

use db::AdminPool;
use uuid::Uuid;

use crate::auth::password::{burn_password_check, verify_password};
use crate::auth::repository;
use crate::auth::session::{SessionToken, create_session, hash_token};

/// 登录输入。
pub struct LoginInput {
    /// 登录邮箱。
    pub email: String,
    /// 明文密码，只用于校验。
    pub password: String,
    /// 请求携带的旧会话 token，登录时吊销。
    pub current_token: Option<String>,
}

/// 登录成功后的会话与租户上下文。
#[derive(Debug)]
pub struct LoginResult {
    /// 用户主键。
    pub user_id: Uuid,
    /// 当前租户；用户尚未加入租户时为空。
    pub tenant_id: Option<Uuid>,
    /// 新会话 token。
    pub session_token: SessionToken,
}

/// 登录错误。
#[derive(Debug, thiserror::Error)]
pub enum LoginError {
    /// 邮箱或密码错误；不区分用户是否存在。
    #[error("邮箱或密码错误")]
    InvalidCredentials,
    /// 密码哈希或内部处理失败。
    #[error("登录内部错误")]
    Internal,
    /// 数据库操作失败。
    #[error("登录数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}

/// 校验凭据，吊销旧会话并创建新会话。
///
/// 用户不存在时仍执行一次密码哈希，避免通过响应时间枚举账号。
/// # Errors
/// 凭据无效时返回 [`LoginError::InvalidCredentials`]；
/// 数据库失败时返回 [`LoginError::Database`]。
pub async fn login(pool: &AdminPool, input: LoginInput) -> Result<LoginResult, LoginError> {
    let email = input.email.trim().to_ascii_lowercase();
    if !is_valid_email(&email) {
        burn_password_check(input.password).await;
        return Err(LoginError::InvalidCredentials);
    }

    let user = repository::find_user(pool.pool(), &email)
        .await
        .map_err(|source| LoginError::Database { source })?;
    let Some(user) = user else {
        burn_password_check(input.password).await;
        return Err(LoginError::InvalidCredentials);
    };
    if user.status != "active" {
        burn_password_check(input.password).await;
        return Err(LoginError::InvalidCredentials);
    }

    let valid = verify_password(input.password, user.password_hash)
        .await
        .map_err(|_| LoginError::Internal)?;
    if !valid {
        return Err(LoginError::InvalidCredentials);
    }

    let mut transaction = pool
        .pool()
        .begin()
        .await
        .map_err(|source| LoginError::Database { source })?;

    if let Some(token) = input.current_token {
        repository::revoke_by_hash(transaction.as_mut(), &hash_token(&token))
            .await
            .map_err(|source| LoginError::Database { source })?;
    }

    let tenant_id = repository::first_tenant(transaction.as_mut(), user.id)
        .await
        .map_err(|source| LoginError::Database { source })?;
    let session_token = create_session(&mut transaction, user.id, tenant_id)
        .await
        .map_err(|source| LoginError::Database { source })?;

    transaction
        .commit()
        .await
        .map_err(|source| LoginError::Database { source })?;

    Ok(LoginResult {
        user_id: user.id,
        tenant_id,
        session_token,
    })
}

fn is_valid_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    !local.is_empty()
        && !domain.is_empty()
        && domain.contains('.')
        && email.len() <= 254
        && !email.chars().any(char::is_whitespace)
}
