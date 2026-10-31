//! 班级服务：名称校验与删除课次约束。

use uuid::Uuid;

use crate::class::model::{Class, ClassError, ClassInput};
use crate::class::repository;

const MAX_NAME_CHARS: usize = 50;

/// 列出当前租户全部班级。
///
/// # Errors
/// 数据库操作失败时返回 [`ClassError`]。
pub async fn list_classes(
    pool: &db::TenantPool,
    tenant_id: Uuid,
) -> Result<Vec<Class>, ClassError> {
    let mut transaction = begin(pool, tenant_id).await?;
    repository::list(transaction.transaction_mut(), tenant_id)
        .await
        .map_err(database)
}

/// 创建班级。
///
/// # Errors
/// 名称为空或过长、数据库操作失败时返回 [`ClassError`]。
pub async fn create_class(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    input: ClassInput<'_>,
) -> Result<Class, ClassError> {
    let name = normalized_name(input.name)?;
    let subject = normalized_optional(input.subject);
    let mut transaction = begin(pool, tenant_id).await?;
    let class = repository::create(
        transaction.transaction_mut(),
        tenant_id,
        ClassInput {
            name: &name,
            subject: subject.as_deref(),
        },
    )
    .await
    .map_err(database)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(class)
}

/// 更新班级。
///
/// # Errors
/// 班级不存在、名称无效或数据库操作失败时返回 [`ClassError`]。
pub async fn update_class(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    class_id: Uuid,
    input: ClassInput<'_>,
) -> Result<Class, ClassError> {
    let name = normalized_name(input.name)?;
    let subject = normalized_optional(input.subject);
    let mut transaction = begin(pool, tenant_id).await?;
    let class = repository::update(
        transaction.transaction_mut(),
        tenant_id,
        class_id,
        ClassInput {
            name: &name,
            subject: subject.as_deref(),
        },
    )
    .await
    .map_err(database)?
    .ok_or(ClassError::NotFound)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(class)
}

/// 删除班级；存在课次时拒绝，不级联删除课次。
///
/// 班级行锁与课次外键的 KEY SHARE 锁互斥，防止检查后并发插入课次。
///
/// # Errors
/// 班级不存在、仍有课次或数据库操作失败时返回 [`ClassError`]。
pub async fn delete_class(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    class_id: Uuid,
) -> Result<(), ClassError> {
    let mut transaction = begin(pool, tenant_id).await?;
    if repository::lock_for_delete(transaction.transaction_mut(), tenant_id, class_id)
        .await
        .map_err(database)?
        .is_none()
    {
        return Err(ClassError::NotFound);
    }
    if repository::has_lessons(transaction.transaction_mut(), tenant_id, class_id)
        .await
        .map_err(database)?
    {
        return Err(ClassError::HasLessons);
    }
    if !repository::delete(transaction.transaction_mut(), tenant_id, class_id)
        .await
        .map_err(database)?
    {
        return Err(ClassError::NotFound);
    }
    transaction.commit().await.map_err(transaction_error)?;
    Ok(())
}

fn normalized_name(name: &str) -> Result<String, ClassError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(ClassError::InvalidName);
    }
    Ok(name.to_owned())
}

fn normalized_optional(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

async fn begin(
    pool: &db::TenantPool,
    tenant_id: Uuid,
) -> Result<db::TenantTransaction, ClassError> {
    pool.begin(tenant_id).await.map_err(transaction_error)
}

fn transaction_error(source: db::DbError) -> ClassError {
    ClassError::Transaction { source }
}

fn database(source: sqlx::Error) -> ClassError {
    ClassError::Database { source }
}
