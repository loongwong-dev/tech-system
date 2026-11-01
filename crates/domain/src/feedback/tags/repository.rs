//! 标签仓储：读取学员最近一次使用的标签组合。

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// 学员最近一次反馈的标签组合；`None` 表示该学员还没有任何反馈。
///
/// 「最近」按课次日期倒序，同日按反馈写入时间倒序，再用 id 兜底，保证结果稳定。
pub(crate) async fn latest_tags(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<Option<Vec<String>>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT f.knowledge_tags
        FROM feedbacks f
        JOIN lessons l ON l.id = f.lesson_id
        WHERE f.student_id = $1 AND f.tenant_id = $2
        ORDER BY l.lesson_date DESC, f.updated_at DESC, f.id DESC
        LIMIT 1
        "#,
        student_id,
        tenant_id,
    )
    .fetch_optional(&mut **transaction)
    .await
}
