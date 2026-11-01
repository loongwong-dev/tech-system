//! 课次服务：输入校验、班级归属与分页边界。

use uuid::Uuid;

use crate::lesson::model::{
    Lesson, LessonCursor, LessonError, LessonFilter, LessonInput, LessonPage,
};
use crate::lesson::repository;

/// 默认每页数量。
pub const DEFAULT_LIMIT: i64 = 20;
/// 每页数量上限。
pub const MAX_LIMIT: i64 = 100;

/// 课次主题长度上限。
pub const MAX_TOPIC_CHARS: usize = 200;

/// 按日期范围列出当前租户课次。
///
/// # Errors
/// 日期范围或分页数量非法、数据库操作失败时返回 [`LessonError`]。
pub async fn list_lessons(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    filter: LessonFilter,
) -> Result<LessonPage, LessonError> {
    validate_limit(filter.limit)?;
    if let (Some(from), Some(to)) = (filter.from, filter.to)
        && from > to
    {
        return Err(LessonError::InvalidDateRange);
    }

    let mut transaction = begin(pool, tenant_id).await?;
    let mut items = repository::list(
        transaction.transaction_mut(),
        tenant_id,
        filter,
        filter.limit + 1,
    )
    .await
    .map_err(database)?;

    let page_size = usize::try_from(filter.limit).map_err(|_| LessonError::InvalidLimit)?;
    let has_more = items.len() > page_size;
    if has_more {
        items.truncate(page_size);
    }
    let next_cursor = if has_more {
        items.last().map(|lesson| LessonCursor {
            lesson_date: lesson.lesson_date,
            id: lesson.id,
        })
    } else {
        None
    };

    Ok(LessonPage { items, next_cursor })
}

/// 创建课次；班级必须属于当前租户。
///
/// # Errors
/// 班级不存在、主题过长或数据库操作失败时返回 [`LessonError`]。
pub async fn create_lesson(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    input: LessonInput<'_>,
) -> Result<Lesson, LessonError> {
    let topic = normalized_topic(input.topic)?;
    let mut transaction = begin(pool, tenant_id).await?;
    if repository::lock_for_create(transaction.transaction_mut(), tenant_id, input.class_id)
        .await
        .map_err(database)?
        .is_none()
    {
        return Err(LessonError::ClassNotFound);
    }
    let lesson = repository::create(
        transaction.transaction_mut(),
        tenant_id,
        LessonInput {
            class_id: input.class_id,
            lesson_date: input.lesson_date,
            topic: topic.as_deref(),
        },
    )
    .await
    .map_err(database)?;
    transaction.commit().await.map_err(transaction_error)?;
    Ok(lesson)
}

fn validate_limit(limit: i64) -> Result<(), LessonError> {
    if (1..=MAX_LIMIT).contains(&limit) {
        Ok(())
    } else {
        Err(LessonError::InvalidLimit)
    }
}

fn normalized_topic(value: Option<&str>) -> Result<Option<String>, LessonError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_TOPIC_CHARS {
        return Err(LessonError::InvalidTopic);
    }
    Ok(Some(value.to_owned()))
}

async fn begin(
    pool: &db::TenantPool,
    tenant_id: Uuid,
) -> Result<db::TenantTransaction, LessonError> {
    pool.begin(tenant_id).await.map_err(transaction_error)
}

fn transaction_error(source: db::DbError) -> LessonError {
    LessonError::Transaction { source }
}

fn database(source: sqlx::Error) -> LessonError {
    LessonError::Database { source }
}
