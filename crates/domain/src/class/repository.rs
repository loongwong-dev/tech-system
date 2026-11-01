//! 班级仓储：删除前锁定班级，避免并发插入课次后被级联删除。

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::class::model::{Class, ClassInput};

pub(crate) async fn list(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
) -> Result<Vec<Class>, sqlx::Error> {
    sqlx::query_as!(
        Class,
        r#"
        SELECT id, name, subject, created_at
        FROM classes
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
    input: ClassInput<'_>,
) -> Result<Class, sqlx::Error> {
    sqlx::query_as!(
        Class,
        r#"
        INSERT INTO classes (tenant_id, name, subject)
        VALUES ($1, $2, $3)
        RETURNING id, name, subject, created_at
        "#,
        tenant_id,
        input.name,
        input.subject,
    )
    .fetch_one(&mut **transaction)
    .await
}

pub(crate) async fn update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    class_id: Uuid,
    input: ClassInput<'_>,
) -> Result<Option<Class>, sqlx::Error> {
    sqlx::query_as!(
        Class,
        r#"
        UPDATE classes
        SET name = $3, subject = $4
        WHERE id = $1 AND tenant_id = $2 AND archived_at IS NULL
        RETURNING id, name, subject, created_at
        "#,
        class_id,
        tenant_id,
        input.name,
        input.subject,
    )
    .fetch_optional(&mut **transaction)
    .await
}

pub(crate) async fn archive(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    class_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE classes
        SET archived_at = now()
        WHERE id = $1 AND tenant_id = $2 AND archived_at IS NULL
        "#,
        class_id,
        tenant_id,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn lock_for_delete(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    class_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT id
        FROM classes
        WHERE id = $1 AND tenant_id = $2
        FOR UPDATE
        "#,
        class_id,
        tenant_id,
    )
    .fetch_optional(&mut **transaction)
    .await
}

pub(crate) async fn has_lessons(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    class_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM lessons
          WHERE class_id = $1 AND tenant_id = $2
        ) AS "exists!"
        "#,
        class_id,
        tenant_id,
    )
    .fetch_one(&mut **transaction)
    .await
}

pub(crate) async fn delete(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    class_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM classes
        WHERE id = $1 AND tenant_id = $2
        "#,
        class_id,
        tenant_id,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(result.rows_affected() == 1)
}
