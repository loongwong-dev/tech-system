//! 注册业务规则与事务边界。

use db::AdminPool;
use uuid::Uuid;

use crate::auth::{create_session, hash_password, validate_password};
use crate::invite::consume_invite_code_in_transaction;
use crate::registration::error::RegistrationError;
use crate::registration::model::{RegisteredAccount, RegistrationInput};
use crate::registration::repository;

/// 使用邀请码注册个人租户。
///
/// 邀请码消耗、用户、租户、owner 关系与会话在同一事务内完成；
/// 任一步失败都会回滚，邀请码不会被白白消耗。
/// # Errors
/// 邮箱格式、密码策略、邀请码、邮箱唯一性或数据库操作失败时返回对应错误。
pub async fn register(
    pool: &AdminPool,
    input: RegistrationInput,
) -> Result<RegisteredAccount, RegistrationError> {
    let email = normalize_email(&input.email)?;
    validate_password(&input.password)?;
    let password_hash = hash_password(input.password).await?;

    let tenant_name = "我的租户";
    let tenant_slug = format!("tenant-{}", Uuid::new_v4().simple());
    let mut transaction = pool
        .pool()
        .begin()
        .await
        .map_err(|source| RegistrationError::Database { source })?;

    consume_invite_code_in_transaction(&mut transaction, &input.invite_code).await?;

    let tenant_id = repository::create_tenant(&mut transaction, tenant_name, &tenant_slug)
        .await
        .map_err(|source| RegistrationError::Database { source })?;
    let user_id = repository::create_user(&mut transaction, &email, &password_hash)
        .await
        .map_err(map_user_insert_error)?;
    repository::create_membership(&mut transaction, tenant_id, user_id)
        .await
        .map_err(|source| RegistrationError::Database { source })?;
    let session_token = create_session(&mut transaction, user_id, tenant_id)
        .await
        .map_err(|source| RegistrationError::Database { source })?;

    transaction
        .commit()
        .await
        .map_err(|source| RegistrationError::Database { source })?;

    Ok(RegisteredAccount {
        user_id,
        tenant_id,
        session_token,
    })
}

fn normalize_email(input: &str) -> Result<String, RegistrationError> {
    let email = input.trim().to_ascii_lowercase();
    let (local, domain) = email
        .split_once('@')
        .ok_or(RegistrationError::InvalidEmail)?;
    if local.is_empty()
        || domain.is_empty()
        || !domain.contains('.')
        || email.len() > 254
        || email.chars().any(char::is_whitespace)
    {
        return Err(RegistrationError::InvalidEmail);
    }
    Ok(email)
}

fn map_user_insert_error(error: sqlx::Error) -> RegistrationError {
    if let sqlx::Error::Database(database_error) = &error
        && database_error.code().as_deref() == Some("23505")
        && database_error.constraint() == Some("users_email_lower_idx")
    {
        return RegistrationError::EmailExists;
    }
    RegistrationError::Database { source: error }
}
