//! 学员仓储：所有查询显式带租户条件。

use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::student::model::{Student, StudentInput};

pub(crate) async fn list(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
) -> Result<Vec<Student>, sqlx::Error> {
    sqlx::query_as!(
        Student,
        r#"
        SELECT id, name, grade, subject, created_at
        FROM students
        WHERE tenant_id = $1 AND archived_at IS NULL
        ORDER BY created_at DESC, id
        "#,
        tenant_id,
    )
    .fetch_all(&mut **transaction)
    .await
}

pub(crate) async fn create(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    input: StudentInput<'_>,
) -> Result<Student, sqlx::Error> {
    sqlx::query_as!(
        Student,
        r#"
        INSERT INTO students (tenant_id, name, grade, subject)
        VALUES ($1, $2, $3, $4)
        RETURNING id, name, grade, subject, created_at
        "#,
        tenant_id,
        input.name,
        input.grade,
        input.subject,
    )
    .fetch_one(&mut **transaction)
    .await
}

pub(crate) async fn update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
    input: StudentInput<'_>,
) -> Result<Option<Student>, sqlx::Error> {
    sqlx::query_as!(
        Student,
        r#"
        UPDATE students
        SET name = $3, grade = $4, subject = $5
        WHERE id = $1 AND tenant_id = $2 AND archived_at IS NULL
        RETURNING id, name, grade, subject, created_at
        "#,
        student_id,
        tenant_id,
        input.name,
        input.grade,
        input.subject,
    )
    .fetch_optional(&mut **transaction)
    .await
}

pub(crate) async fn archive(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE students
        SET archived_at = now()
        WHERE id = $1 AND tenant_id = $2 AND archived_at IS NULL
        "#,
        student_id,
        tenant_id,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn lock_for_delete(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<Option<Student>, sqlx::Error> {
    sqlx::query_as!(
        Student,
        r#"
        SELECT id, name, grade, subject, created_at
        FROM students
        WHERE id = $1 AND tenant_id = $2
        FOR UPDATE
        "#,
        student_id,
        tenant_id,
    )
    .fetch_optional(&mut **transaction)
    .await
}

pub(crate) async fn feedback_count(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT count(*)::bigint AS "count!"
        FROM feedbacks
        WHERE student_id = $1 AND tenant_id = $2
        "#,
        student_id,
        tenant_id,
    )
    .fetch_one(&mut **transaction)
    .await
}

pub(crate) async fn insert_delete_audit(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    actor_id: Uuid,
    student_id: Uuid,
    before: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO audit_logs (tenant_id, actor_id, action, target, before, after)
        VALUES ($1, $2, 'student.deleted', $3, $4, NULL)
        "#,
        tenant_id,
        actor_id,
        student_id.to_string(),
        before,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

pub(crate) async fn delete(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM students
        WHERE id = $1 AND tenant_id = $2
        "#,
        student_id,
        tenant_id,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}
