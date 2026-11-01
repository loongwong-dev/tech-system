//! 反馈草稿仓储：只在租户事务内执行 SQL。

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::feedback::draft::model::{DraftIdentity, DraftInput, FeedbackDraft};

/// 课次是否存在且属于当前租户。
///
/// 必须显式校验：外键不区分租户，缺这一步可以把自己的反馈挂到别家课次上。
pub(crate) async fn lesson_exists(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM lessons WHERE id = $1 AND tenant_id = $2
        ) AS "exists!"
        "#,
        lesson_id,
        tenant_id,
    )
    .fetch_one(&mut **transaction)
    .await
}

/// 学员是否存在且属于当前租户；已归档学员仍允许写入，历史课次需要补录。
pub(crate) async fn student_exists(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM students WHERE id = $1 AND tenant_id = $2
        ) AS "exists!"
        "#,
        student_id,
        tenant_id,
    )
    .fetch_one(&mut **transaction)
    .await
}

/// 按 id 读取草稿身份；跨租户返回 `None`（RLS 过滤）。
pub(crate) async fn find_identity(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    draft_id: Uuid,
) -> Result<Option<DraftIdentity>, sqlx::Error> {
    sqlx::query_as!(
        DraftIdentity,
        r#"
        SELECT lesson_id, student_id, status
        FROM feedbacks
        WHERE id = $1 AND tenant_id = $2
        "#,
        draft_id,
        tenant_id,
    )
    .fetch_optional(&mut **transaction)
    .await
}

/// 幂等写入草稿。
///
/// 主键未占用时插入；命中 `(lesson_id, student_id)` 唯一约束时更新已有行并
/// 返回它的 id。并发提交同一组合会串行在唯一索引上，因此不会产生重复行。
///
/// 返回 `None` 表示命中约束但目标行不是 `draft`（`WHERE` 过滤掉了更新）。
pub(crate) async fn upsert(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    draft_id: Uuid,
    input: DraftInput,
) -> Result<Option<FeedbackDraft>, sqlx::Error> {
    sqlx::query_as!(
        FeedbackDraft,
        r#"
        INSERT INTO feedbacks (
          id, tenant_id, lesson_id, student_id, notes, ratings, knowledge_tags, homework
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (lesson_id, student_id) DO UPDATE
          SET notes = EXCLUDED.notes,
              ratings = EXCLUDED.ratings,
              knowledge_tags = EXCLUDED.knowledge_tags,
              homework = EXCLUDED.homework,
              updated_at = now()
          WHERE feedbacks.status = 'draft'
        RETURNING id, lesson_id, student_id, notes, ratings, knowledge_tags, homework,
                  status, updated_at
        "#,
        draft_id,
        tenant_id,
        input.lesson_id,
        input.student_id,
        input.notes,
        input.ratings,
        input.knowledge_tags.as_slice(),
        input.homework,
    )
    .fetch_optional(&mut **transaction)
    .await
}

/// 列出课次内全部反馈，按学员排序，保证前端合并结果稳定。
pub(crate) async fn list_by_lesson(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<Vec<FeedbackDraft>, sqlx::Error> {
    sqlx::query_as!(
        FeedbackDraft,
        r#"
        SELECT id, lesson_id, student_id, notes, ratings, knowledge_tags, homework,
               status, updated_at
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
