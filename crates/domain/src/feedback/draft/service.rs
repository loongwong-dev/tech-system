//! 反馈草稿服务：输入校验、租户边界与幂等写入。

use serde_json::Value;
use uuid::Uuid;

use crate::feedback::draft::model::{DraftError, DraftIdentity, DraftInput, FeedbackDraft};
use crate::feedback::draft::repository;

/// 教师原文长度上限（字符数）。
pub const MAX_NOTES_CHARS: usize = 2000;
/// 单次提交的知识点标签数量上限。
pub const MAX_TAGS: usize = 50;
/// 单个知识点标签的长度上限（字符数）。
pub const MAX_TAG_CHARS: usize = 80;
/// `ratings` 与 `homework` 序列化后的字节上限。
pub const MAX_JSON_BYTES: usize = 4096;

/// 主键约束名，用于把 id 复用与「同一组合重复写入」区分开。
const PRIMARY_KEY_CONSTRAINT: &str = "feedbacks_pkey";

/// 列出课次内全部反馈。
///
/// 返回 `None` 表示课次不存在或不属于当前租户；`Some(vec![])` 表示课次存在但没有反馈。
///
/// # Errors
/// 数据库操作失败时返回 [`DraftError`]。
pub async fn list_lesson_drafts(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<Option<Vec<FeedbackDraft>>, DraftError> {
    let mut transaction = begin(pool, tenant_id).await?;
    if !repository::lesson_exists(transaction.transaction_mut(), tenant_id, lesson_id)
        .await
        .map_err(database)?
    {
        return Ok(None);
    }
    let drafts = repository::list_by_lesson(transaction.transaction_mut(), tenant_id, lesson_id)
        .await
        .map_err(database)?;
    Ok(Some(drafts))
}

/// 幂等写入一份草稿。
///
/// `draft_id` 由客户端生成：主键未占用时插入；命中 `(lesson_id, student_id)`
/// 唯一约束时更新该组合已有的草稿。并发提交同一组合由唯一索引串行化，
/// 因此不会产生重复行；调用方必须以后端返回的 id 为准。
///
/// # Errors
/// 课次或学员不属于当前租户时返回 `LessonNotFound` / `StudentNotFound`；
/// 目标反馈不是 `draft` 时返回 `InvalidTransition`；
/// 路径 id 被同一租户的其他学员占用时返回 `IdReused`；
/// 字段非法时返回对应的 `Invalid*`；数据库操作失败时返回 `Transaction` / `Database`。
pub async fn save_draft(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    draft_id: Uuid,
    input: DraftInput,
) -> Result<FeedbackDraft, DraftError> {
    validate(&input)?;
    let mut transaction = begin(pool, tenant_id).await?;
    if !repository::lesson_exists(transaction.transaction_mut(), tenant_id, input.lesson_id)
        .await
        .map_err(database)?
    {
        return Err(DraftError::LessonNotFound);
    }
    if !repository::student_exists(transaction.transaction_mut(), tenant_id, input.student_id)
        .await
        .map_err(database)?
    {
        return Err(DraftError::StudentNotFound);
    }
    // 先确认路径 id 的归属：同一租户内被别的学员占用时报 409，而不是让主键冲突变成 500。
    if let Some(identity) =
        repository::find_identity(transaction.transaction_mut(), tenant_id, draft_id)
            .await
            .map_err(database)?
    {
        check_identity(&identity, &input)?;
    }

    let saved =
        match repository::upsert(transaction.transaction_mut(), tenant_id, draft_id, input).await {
            Ok(saved) => saved,
            // 主键冲突说明该 id 属于别的租户：RLS 看不到对方，按契约返回 404 而不是暴露存在性。
            Err(error) if is_primary_key_conflict(&error) => return Err(DraftError::NotFound),
            Err(error) => return Err(database(error)),
        };
    // 未返回行只可能是并发事务在本事务读身份之后把状态改成了非 draft。
    let draft = saved.ok_or(DraftError::InvalidTransition)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(draft)
}

/// 路径 id 已被占用时，只有指向同一份草稿才允许继续写入。
fn check_identity(identity: &DraftIdentity, input: &DraftInput) -> Result<(), DraftError> {
    if identity.lesson_id != input.lesson_id || identity.student_id != input.student_id {
        return Err(DraftError::IdReused);
    }
    if identity.status != "draft" {
        return Err(DraftError::InvalidTransition);
    }
    Ok(())
}

fn validate(input: &DraftInput) -> Result<(), DraftError> {
    if input.notes.chars().count() > MAX_NOTES_CHARS {
        return Err(DraftError::InvalidNotes);
    }
    validate_tags(&input.knowledge_tags)?;
    validate_json(&input.ratings).map_err(|()| DraftError::InvalidRatings)?;
    validate_json(&input.homework).map_err(|()| DraftError::InvalidHomework)?;
    Ok(())
}

/// 校验标签数量与单个标签：空白标签与超长标签一律拒绝。
///
/// 标签是否属于内置维度由标签体系（#33）负责，这里只做形状与长度校验。
fn validate_tags(tags: &[String]) -> Result<(), DraftError> {
    if tags.len() > MAX_TAGS {
        return Err(DraftError::InvalidTags);
    }
    let invalid = tags.iter().any(|tag| {
        let trimmed = tag.trim();
        trimmed.is_empty() || trimmed.chars().count() > MAX_TAG_CHARS
    });
    if invalid {
        return Err(DraftError::InvalidTags);
    }
    Ok(())
}

/// 只接受 JSON 对象并限制体积：数组、标量与大对象都不是合法草稿字段。
fn validate_json(value: &Value) -> Result<(), ()> {
    if !value.is_object() {
        return Err(());
    }
    let Ok(bytes) = serde_json::to_vec(value) else {
        return Err(());
    };
    if bytes.len() > MAX_JSON_BYTES {
        return Err(());
    }
    Ok(())
}

/// 主键冲突说明路径 id 已被占用（可能是别的租户）。
fn is_primary_key_conflict(error: &sqlx::Error) -> bool {
    matches!(error, sqlx::Error::Database(database) if database.constraint() == Some(PRIMARY_KEY_CONSTRAINT))
}

async fn begin(
    pool: &db::TenantPool,
    tenant_id: Uuid,
) -> Result<db::TenantTransaction, DraftError> {
    pool.begin(tenant_id).await.map_err(transaction_error)
}

fn transaction_error(source: db::DbError) -> DraftError {
    DraftError::Transaction { source }
}

fn database(source: sqlx::Error) -> DraftError {
    DraftError::Database { source }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use crate::feedback::draft::model::{DraftError, DraftIdentity, DraftInput};

    use super::{
        MAX_JSON_BYTES, MAX_NOTES_CHARS, MAX_TAG_CHARS, MAX_TAGS, check_identity, validate,
    };

    fn input() -> DraftInput {
        DraftInput {
            lesson_id: Uuid::new_v4(),
            student_id: Uuid::new_v4(),
            notes: "今天讲判别式".to_owned(),
            ratings: json!({"课堂表现": "专注投入"}),
            knowledge_tags: vec!["专注投入".to_owned()],
            homework: json!({}),
        }
    }

    #[test]
    fn validate_accepts_well_formed_input() {
        assert!(validate(&input()).is_ok());
    }

    #[test]
    fn validate_rejects_out_of_range_fields() {
        let long_notes = "字".repeat(MAX_NOTES_CHARS + 1);
        let mut notes = input();
        notes.notes = long_notes;
        assert!(matches!(validate(&notes), Err(DraftError::InvalidNotes)));

        let mut tags = input();
        tags.knowledge_tags = vec!["标签".to_owned(); MAX_TAGS + 1];
        assert!(matches!(validate(&tags), Err(DraftError::InvalidTags)));

        let mut long_tag = input();
        long_tag.knowledge_tags = vec!["标".repeat(MAX_TAG_CHARS + 1)];
        assert!(matches!(validate(&long_tag), Err(DraftError::InvalidTags)));

        let mut blank_tag = input();
        blank_tag.knowledge_tags = vec!["   ".to_owned()];
        assert!(matches!(validate(&blank_tag), Err(DraftError::InvalidTags)));

        let mut array_ratings = input();
        array_ratings.ratings = json!(["专注投入"]);
        assert!(matches!(
            validate(&array_ratings),
            Err(DraftError::InvalidRatings)
        ));

        let oversized = "a".repeat(MAX_JSON_BYTES);
        let mut big_homework = input();
        big_homework.homework = json!({"note": oversized});
        assert!(matches!(
            validate(&big_homework),
            Err(DraftError::InvalidHomework)
        ));
    }

    #[test]
    fn identity_guards_id_reuse_and_state() {
        let draft = input();
        let identity = |student_id: Uuid, status: &str| DraftIdentity {
            lesson_id: draft.lesson_id,
            student_id,
            status: status.to_owned(),
        };

        assert!(check_identity(&identity(draft.student_id, "draft"), &draft).is_ok());
        assert!(matches!(
            check_identity(&identity(Uuid::new_v4(), "draft"), &draft),
            Err(DraftError::IdReused)
        ));
        assert!(matches!(
            check_identity(&identity(draft.student_id, "generated"), &draft),
            Err(DraftError::InvalidTransition)
        ));
    }
}
