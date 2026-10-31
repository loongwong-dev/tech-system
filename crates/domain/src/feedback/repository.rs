//! 反馈仓储：租户事务内的读取与生成结果写入。

use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::feedback::model::Feedback;

#[derive(sqlx::FromRow)]
struct LessonStats {
    existed: bool,
    count: i64,
}

/// 锁定并读取一条反馈。
pub(crate) async fn feedback_for_update(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    feedback_id: Uuid,
) -> Result<Option<Feedback>, sqlx::Error> {
    sqlx::query_as!(
        Feedback,
        r#"
        SELECT id, lesson_id, notes, knowledge_tags, rendered, status, regen_count
        FROM feedbacks
        WHERE id = $1 AND tenant_id = $2
        FOR UPDATE
        "#,
        feedback_id,
        tenant_id,
    )
    .fetch_optional(&mut **transaction)
    .await
}

/// 读取课次内全部反馈 ID，供批量任务使用。
pub(crate) async fn lesson_feedback_ids(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT id
        FROM feedbacks
        WHERE lesson_id = $1 AND tenant_id = $2
        ORDER BY student_id, id
        "#,
        lesson_id,
        tenant_id,
    )
    .fetch_all(&mut **transaction)
    .await
}

/// 课次是否存在，以及反馈数量。
pub(crate) async fn lesson_feedback_count(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<(bool, i64), sqlx::Error> {
    let stats = sqlx::query_as!(
        LessonStats,
        r#"
        SELECT
          EXISTS (
            SELECT 1 FROM lessons WHERE id = $1 AND tenant_id = $2
          ) AS "existed!",
          (
            SELECT count(*) FROM feedbacks WHERE lesson_id = $1 AND tenant_id = $2
          )::bigint AS "count!"
        "#,
        lesson_id,
        tenant_id,
    )
    .fetch_one(&mut **transaction)
    .await?;
    Ok((stats.existed, stats.count))
}

/// 一次生成写入所需的字段。
pub(crate) struct GeneratedContent<'a> {
    pub(crate) rendered: &'a Value,
    pub(crate) regen_count: i16,
    pub(crate) prompt_version: &'a str,
}

/// 写入生成结果并推进状态。
pub(crate) async fn mark_generated(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    feedback_id: Uuid,
    content: GeneratedContent<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE feedbacks
        SET rendered = $3,
            status = 'generated',
            generated_by = 'template',
            prompt_version = $4,
            regen_count = $5,
            updated_at = now()
        WHERE id = $1 AND tenant_id = $2
        "#,
        feedback_id,
        tenant_id,
        content.rendered,
        content.prompt_version,
        content.regen_count,
    )
    .execute(&mut **transaction)
    .await?;
    Ok(())
}
