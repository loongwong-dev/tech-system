//! 学员服务：校验、归档、删除确认与同事务审计。

use serde_json::json;
use uuid::Uuid;

use crate::student::model::{DeletedStudent, Student, StudentError, StudentInput};
use crate::student::repository;

const MAX_NAME_CHARS: usize = 50;

/// 列出未归档学员。
///
/// # Errors
/// 数据库操作失败时返回 [`StudentError`]。
pub async fn list_students(
    pool: &db::TenantPool,
    tenant_id: Uuid,
) -> Result<Vec<Student>, StudentError> {
    let mut transaction = begin(pool, tenant_id).await?;
    repository::list(transaction.transaction_mut(), tenant_id)
        .await
        .map_err(database)
}

/// 创建学员。
///
/// # Errors
/// 姓名为空或过长、数据库操作失败时返回 [`StudentError`]。
pub async fn create_student(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    input: StudentInput<'_>,
) -> Result<Student, StudentError> {
    let name = normalized_name(input.name)?;
    let grade = normalized_optional(input.grade);
    let subject = normalized_optional(input.subject);
    let mut transaction = begin(pool, tenant_id).await?;
    let student = repository::create(
        transaction.transaction_mut(),
        tenant_id,
        StudentInput {
            name: &name,
            grade: grade.as_deref(),
            subject: subject.as_deref(),
        },
    )
    .await
    .map_err(database)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(student)
}

/// 更新学员。
///
/// # Errors
/// 学员不存在、姓名无效或数据库操作失败时返回 [`StudentError`]。
pub async fn update_student(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    student_id: Uuid,
    input: StudentInput<'_>,
) -> Result<Student, StudentError> {
    let name = normalized_name(input.name)?;
    let grade = normalized_optional(input.grade);
    let subject = normalized_optional(input.subject);
    let mut transaction = begin(pool, tenant_id).await?;
    let student = repository::update(
        transaction.transaction_mut(),
        tenant_id,
        student_id,
        StudentInput {
            name: &name,
            grade: grade.as_deref(),
            subject: subject.as_deref(),
        },
    )
    .await
    .map_err(database)?
    .ok_or(StudentError::NotFound)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(student)
}

/// 归档学员；归档后不再出现在列表中。
///
/// # Errors
/// 学员不存在或数据库操作失败时返回 [`StudentError`]。
pub async fn archive_student(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<(), StudentError> {
    let mut transaction = begin(pool, tenant_id).await?;
    if !repository::archive(transaction.transaction_mut(), tenant_id, student_id)
        .await
        .map_err(database)?
    {
        return Err(StudentError::NotFound);
    }
    transaction.commit().await.map_err(transaction_error)?;
    Ok(())
}

/// 删除学员及其反馈、分享物，并在同一事务写入审计。
///
/// `confirmation` 必须与当前姓名完全一致（仅忽略首尾空白）。
///
/// # Errors
/// 学员不存在、确认姓名不匹配或数据库操作失败时返回 [`StudentError`]。
pub async fn delete_student(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    student_id: Uuid,
    confirmation: &str,
) -> Result<DeletedStudent, StudentError> {
    let mut transaction = begin(pool, tenant_id).await?;
    let student = repository::lock_for_delete(transaction.transaction_mut(), tenant_id, student_id)
        .await
        .map_err(database)?
        .ok_or(StudentError::NotFound)?;

    if confirmation.trim() != student.name {
        return Err(StudentError::NameMismatch);
    }

    let feedback_count =
        repository::feedback_count(transaction.transaction_mut(), tenant_id, student_id)
            .await
            .map_err(database)?;
    let before = json!({
        "studentId": student.id,
        "name": student.name,
        "feedbackCount": feedback_count,
    });
    repository::insert_delete_audit(
        transaction.transaction_mut(),
        tenant_id,
        actor_id,
        student_id,
        &before,
    )
    .await
    .map_err(database)?;
    if !repository::delete(transaction.transaction_mut(), tenant_id, student_id)
        .await
        .map_err(database)?
    {
        return Err(StudentError::NotFound);
    }
    transaction.commit().await.map_err(transaction_error)?;

    Ok(DeletedStudent {
        id: student_id,
        feedback_count,
    })
}

fn normalized_name(name: &str) -> Result<String, StudentError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(StudentError::InvalidName);
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
) -> Result<db::TenantTransaction, StudentError> {
    pool.begin(tenant_id).await.map_err(transaction_error)
}

fn transaction_error(source: db::DbError) -> StudentError {
    StudentError::Transaction { source }
}

fn database(source: sqlx::Error) -> StudentError {
    StudentError::Database { source }
}
