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
/// PostgreSQL `unique_violation`。
const UNIQUE_VIOLATION: &str = "23505";
/// PostgreSQL `serialization_failure`。
const SERIALIZATION_FAILURE: &str = "40001";
/// PostgreSQL `deadlock_detected`。
const DEADLOCK_DETECTED: &str = "40P01";
/// 单次请求最多尝试写入的次数：首次失败后在新事务里重试一次。
const MAX_ATTEMPTS: usize = 2;

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
    // 只读事务也显式提交：尽早结束事务，避免连接回池时还要回滚。
    transaction.commit().await.map_err(transaction_error)?;
    Ok(Some(drafts))
}

/// 幂等写入一份草稿。
///
/// `draft_id` 由客户端生成：主键未占用时插入；命中 `(lesson_id, student_id)`
/// 唯一约束时更新该组合已有的草稿并返回该行的真实 id。并发提交同一组合由唯一
/// 索引串行化，因此不会产生重复行；调用方必须以后端返回的 id 为准。
///
/// 字段校验先于租户归属检查，因此超限字段配跨租户课次时返回 422 而不是 404。
///
/// # Errors
/// 字段非法时返回对应的 `Invalid*`（422）；课次或学员不属于当前租户时返回
/// `LessonNotFound` / `StudentNotFound`；目标反馈不是 `draft` 时返回 `InvalidTransition`；
/// 路径 id 被同一租户的其他草稿占用时返回 `IdReused`；
/// 路径 id 与不可见的记录冲突（通常属于其他租户）时返回 `IdConflict`；
/// 开启事务或提交失败时返回 `Transaction`，其他数据库错误返回 `Database`。
pub async fn save_draft(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    draft_id: Uuid,
    input: &DraftInput,
) -> Result<FeedbackDraft, DraftError> {
    validate(input)?;
    let mut attempt = 0;
    loop {
        attempt += 1;
        match save_attempt(pool, tenant_id, draft_id, input).await {
            Ok(draft) => return Ok(draft),
            // 冲突会中止上一个事务，只能在全新事务里重试：让约束重新裁决，
            // 并发写同一 id 的重复提交因此合并到已提交的那一行。
            Err(error) if attempt < MAX_ATTEMPTS && is_retryable(&error) => {}
            Err(error) => return Err(error),
        }
    }
}

/// 完整执行一次「校验归属 → 写草稿」，失败即回滚（事务随返回值离开作用域被丢弃）。
async fn save_attempt(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    draft_id: Uuid,
    input: &DraftInput,
) -> Result<FeedbackDraft, DraftError> {
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
    // 先确认路径 id 的归属：同一租户内被别的草稿占用时报 409，而不是让主键冲突变成 500。
    if let Some(identity) =
        repository::find_identity(transaction.transaction_mut(), tenant_id, draft_id)
            .await
            .map_err(database)?
    {
        check_identity(&identity, input)?;
    }

    let saved =
        match repository::upsert(transaction.transaction_mut(), tenant_id, draft_id, input).await {
            Ok(saved) => saved,
            // 主键冲突：该 id 已存在，但 RLS 之下查不到它，通常属于其他租户。
            // 交给调用方决定重试；重试后仍冲突即按契约返回 404，不暴露存在性。
            Err(error) if is_primary_key_conflict(&error) => {
                return Err(DraftError::IdConflict { source: error });
            }
            Err(error) => return Err(database(error)),
        };
    // 未返回行表示命中的目标行不是 draft：既有反馈已生成，或并发事务在本事务读身份之后改了状态。
    let draft = saved.ok_or(DraftError::InvalidTransition)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(draft)
}

/// 只有并发写引起的失败值得重试：主键冲突、序列化失败与死锁。
///
/// 三者都会中止当前事务，重试必须重新执行完整检查；业务错误一律不重试。
fn is_retryable(error: &DraftError) -> bool {
    match error {
        DraftError::IdConflict { .. } => true,
        DraftError::Database { source } => matches!(
            sqlstate(source).as_deref(),
            Some(SERIALIZATION_FAILURE | DEADLOCK_DETECTED)
        ),
        _ => false,
    }
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
    // 目录校验放在形状校验之后：数量或长度先出错时给出更具体的字段原因。
    crate::feedback::tags::validate(&input.knowledge_tags).map_err(|error| match error {
        crate::feedback::tags::TagInputError::Unknown { name } => DraftError::UnknownTag { name },
        crate::feedback::tags::TagInputError::Duplicate { dimension } => {
            DraftError::TagConflict { dimension }
        }
    })?;
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
    // `Value` 不可能包含 NaN/Inf 等无法序列化的内容，这个分支是防御性的。
    let Ok(bytes) = serde_json::to_vec(value) else {
        return Err(());
    };
    if bytes.len() > MAX_JSON_BYTES {
        return Err(());
    }
    Ok(())
}

/// 路径 id 主键冲突：`unique_violation` 且约束名是主键。
///
/// 同时看 SQLSTATE 与约束名，约束改名或新增其他唯一约束时不会把错误静默降级成 404。
fn is_primary_key_conflict(error: &sqlx::Error) -> bool {
    sqlstate(error).as_deref() == Some(UNIQUE_VIOLATION)
        && matches!(error, sqlx::Error::Database(database) if database.constraint() == Some(PRIMARY_KEY_CONSTRAINT))
}

/// PostgreSQL 错误的 SQLSTATE，非数据库错误返回 `None`。
fn sqlstate(error: &sqlx::Error) -> Option<String> {
    match error {
        sqlx::Error::Database(database) => database.code().map(std::borrow::Cow::into_owned),
        _ => None,
    }
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
        validate_tags,
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
    fn validate_accepts_values_at_the_documented_limits() {
        // 上限按「字符数」而非字节数计：中文两倍于字节数，此处必须通过。
        let mut notes = input();
        notes.notes = "字".repeat(MAX_NOTES_CHARS);
        assert!(validate(&notes).is_ok(), "MAX_NOTES_CHARS 按字符数计");

        // 每个维度各取一个内置标签：组合合法。
        let mut tags = input();
        tags.knowledge_tags = ["专注投入", "按时完成", "基本掌握", "稳步提升", "笔记规范"]
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        assert!(validate(&tags).is_ok());

        // 边界值本身允许：序列化后的字节数正好等于上限。
        let mut ratings = input();
        let filler = "a".repeat(MAX_JSON_BYTES - "{\"k\":\"\"}".len());
        ratings.ratings = json!({ "k": filler });
        let size = serde_json::to_vec(&ratings.ratings)
            .map(|bytes| bytes.len())
            .unwrap_or_default();
        assert_eq!(size, MAX_JSON_BYTES);
        assert!(validate(&ratings).is_ok());

        // 形状校验先于目录校验：超量输入必须给出 InvalidTags，而不是未知标签或维度冲突。
        let mut too_many = input();
        too_many.knowledge_tags = vec!["专注投入".to_owned(); MAX_TAGS + 1];
        assert!(matches!(validate(&too_many), Err(DraftError::InvalidTags)));

        // 形状校验的边界单独固定：目录里的标签都很短，长度上限只能直接测 validate_tags。
        assert!(validate_tags(&["标".repeat(MAX_TAG_CHARS)]).is_ok());
        assert!(matches!(
            validate_tags(&["标".repeat(MAX_TAG_CHARS + 1)]),
            Err(DraftError::InvalidTags)
        ));

        // 标签必须与内置目录完全一致（不做 trim）：带空白视为未知标签。
        let mut padded = input();
        padded.knowledge_tags = vec!["  专注投入  ".to_owned()];
        assert!(matches!(
            validate(&padded),
            Err(DraftError::UnknownTag { .. })
        ));
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
