//! 课次仓储：班级归属校验与 keyset 分页查询。

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::lesson::model::{Lesson, LessonFilter, LessonInput};

/// 以 `FOR KEY SHARE` 读取班级，和删除班级的 `FOR UPDATE` 串行化。
///
/// 返回 `None` 表示班级不存在或不属于当前租户；调用方必须据此返回 404。
pub(crate) async fn lock_for_create(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    class_id: Uuid,
) -> Result<Option<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT id
        FROM classes
        WHERE id = $1 AND tenant_id = $2 AND archived_at IS NULL
        FOR KEY SHARE
        "#,
        class_id,
        tenant_id,
    )
    .fetch_optional(&mut **transaction)
    .await
}

pub(crate) async fn create(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    input: LessonInput<'_>,
) -> Result<Lesson, sqlx::Error> {
    sqlx::query_as!(
        Lesson,
        r#"
        INSERT INTO lessons (tenant_id, class_id, lesson_date, topic)
        VALUES ($1, $2, $3, $4)
        RETURNING id, class_id, lesson_date, topic, status, created_at
        "#,
        tenant_id,
        input.class_id,
        input.lesson_date,
        input.topic,
    )
    .fetch_one(&mut **transaction)
    .await
}

pub(crate) async fn list(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    filter: LessonFilter,
    limit: i64,
) -> Result<Vec<Lesson>, sqlx::Error> {
    let cursor_date = filter.cursor.map(|value| value.lesson_date);
    let cursor_id = filter.cursor.map(|value| value.id);
    sqlx::query_as!(
        Lesson,
        r#"
        SELECT id, class_id, lesson_date, topic, status, created_at
        FROM lessons
        WHERE tenant_id = $1
          AND ($2::date IS NULL OR lesson_date >= $2)
          AND ($3::date IS NULL OR lesson_date <= $3)
          AND ($4::date IS NULL OR (lesson_date, id) < ($4::date, $5::uuid))
        ORDER BY lesson_date DESC, id DESC
        LIMIT $6
        "#,
        tenant_id,
        filter.from,
        filter.to,
        cursor_date,
        cursor_id,
        limit,
    )
    .fetch_all(&mut **transaction)
    .await
}
