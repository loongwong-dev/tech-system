//! 反馈生成服务：单份幂等生成与课次批量生成。

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::feedback::model::{Feedback, GeneratedFeedback, GenerationError};
use crate::feedback::repository::GeneratedContent;
use crate::feedback::{repository, template};

/// 允许重新生成的最大次数。
const MAX_REGENERATIONS: i16 = 3;

/// 生成单份反馈。
///
/// 同一 `idempotency_key` 命中上次结果时直接返回，不增加 `regen_count`。
/// 已生成或已审核的反馈最多重新生成 3 次；已分享或已吊销的反馈拒绝生成。
///
/// # Errors
/// 反馈不存在、状态不允许、超过重生成上限或数据库操作失败时返回错误。
pub async fn generate_single(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    feedback_id: Uuid,
    idempotency_key: Option<&str>,
) -> Result<GeneratedFeedback, GenerationError> {
    let mut transaction = pool
        .begin(tenant_id)
        .await
        .map_err(|source| GenerationError::Transaction { source })?;

    let feedback =
        repository::feedback_for_update(transaction.transaction_mut(), tenant_id, feedback_id)
            .await
            .map_err(|source| GenerationError::Database { source })?
            .ok_or(GenerationError::NotFound)?;

    if let Some(key) = idempotency_key
        && let Some((text, stored_key)) = existing_generation(&feedback)
        && stored_key == key
    {
        transaction
            .commit()
            .await
            .map_err(|source| GenerationError::Transaction { source })?;
        return Ok(GeneratedFeedback {
            feedback_id,
            text,
            generated_by: "template".to_owned(),
            prompt_version: template::PROMPT_VERSION.to_owned(),
            reused: true,
        });
    }

    validate_state(&feedback)?;
    let text = template::render(&feedback);
    let next_regen_count = next_regen_count(&feedback);
    let rendered = rendered_payload(&text, idempotency_key);
    repository::mark_generated(
        transaction.transaction_mut(),
        tenant_id,
        feedback_id,
        GeneratedContent {
            rendered: &rendered,
            regen_count: next_regen_count,
            prompt_version: template::PROMPT_VERSION,
        },
    )
    .await
    .map_err(|source| GenerationError::Database { source })?;
    transaction
        .commit()
        .await
        .map_err(|source| GenerationError::Transaction { source })?;

    Ok(GeneratedFeedback {
        feedback_id,
        text,
        generated_by: "template".to_owned(),
        prompt_version: template::PROMPT_VERSION.to_owned(),
        reused: false,
    })
}

/// 生成课次内全部反馈，返回实际生成的份数。
///
/// 已分享、已吊销或达到重生成上限的反馈跳过，不阻塞其他反馈。
///
/// # Errors
/// 课次没有反馈或数据库操作失败时返回错误。
pub async fn generate_batch(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<usize, GenerationError> {
    let mut transaction = pool
        .begin(tenant_id)
        .await
        .map_err(|source| GenerationError::Transaction { source })?;

    let feedback_ids =
        repository::lesson_feedback_ids(transaction.transaction_mut(), tenant_id, lesson_id)
            .await
            .map_err(|source| GenerationError::Database { source })?;
    if feedback_ids.is_empty() {
        return Err(GenerationError::LessonEmpty);
    }

    let mut generated = 0;
    for feedback_id in feedback_ids {
        let feedback =
            repository::feedback_for_update(transaction.transaction_mut(), tenant_id, feedback_id)
                .await
                .map_err(|source| GenerationError::Database { source })?;
        let Some(feedback) = feedback else {
            continue;
        };
        if !is_generatable(&feedback) {
            continue;
        }

        let text = template::render(&feedback);
        let rendered = rendered_payload(&text, None);
        repository::mark_generated(
            transaction.transaction_mut(),
            tenant_id,
            feedback_id,
            GeneratedContent {
                rendered: &rendered,
                regen_count: next_regen_count(&feedback),
                prompt_version: template::PROMPT_VERSION,
            },
        )
        .await
        .map_err(|source| GenerationError::Database { source })?;
        generated += 1;
    }

    transaction
        .commit()
        .await
        .map_err(|source| GenerationError::Transaction { source })?;
    Ok(generated)
}

/// 统计课次下的反馈数量。
///
/// 返回 `None` 表示课次不存在或不属于当前租户；`Some(0)` 表示课次没有反馈。
///
/// # Errors
/// 数据库操作失败时返回错误。
pub async fn lesson_feedback_count(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    lesson_id: Uuid,
) -> Result<Option<i64>, GenerationError> {
    let mut transaction = pool
        .begin(tenant_id)
        .await
        .map_err(|source| GenerationError::Transaction { source })?;
    let (exists, count) =
        repository::lesson_feedback_count(transaction.transaction_mut(), tenant_id, lesson_id)
            .await
            .map_err(|source| GenerationError::Database { source })?;
    if !exists {
        return Ok(None);
    }
    Ok(Some(count))
}

fn validate_state(feedback: &Feedback) -> Result<(), GenerationError> {
    match feedback.status.as_str() {
        "draft" | "failed" => Ok(()),
        "generated" | "approved" => {
            if feedback.regen_count >= MAX_REGENERATIONS {
                Err(GenerationError::RegenerationLimit)
            } else {
                Ok(())
            }
        }
        _ => Err(GenerationError::InvalidState),
    }
}

fn is_generatable(feedback: &Feedback) -> bool {
    if matches!(feedback.status.as_str(), "generated" | "approved") {
        feedback.regen_count < MAX_REGENERATIONS
    } else {
        !matches!(
            feedback.status.as_str(),
            "shared" | "revoked" | "generating"
        )
    }
}

fn next_regen_count(feedback: &Feedback) -> i16 {
    if matches!(feedback.status.as_str(), "generated" | "approved") {
        feedback.regen_count.saturating_add(1)
    } else {
        feedback.regen_count
    }
}

fn existing_generation(feedback: &Feedback) -> Option<(String, String)> {
    let rendered = feedback.rendered.as_ref()?;
    let text = rendered.get("text")?.as_str()?.to_owned();
    let key = rendered.get("idempotencyKey")?.as_str()?.to_owned();
    Some((text, key))
}

fn rendered_payload(text: &str, idempotency_key: Option<&str>) -> Value {
    let mut rendered = Map::new();
    rendered.insert("text".to_owned(), Value::String(text.to_owned()));
    rendered.insert(
        "generatedBy".to_owned(),
        Value::String("template".to_owned()),
    );
    rendered.insert(
        "promptVersion".to_owned(),
        Value::String(template::PROMPT_VERSION.to_owned()),
    );
    if let Some(key) = idempotency_key {
        rendered.insert("idempotencyKey".to_owned(), Value::String(key.to_owned()));
    }
    Value::Object(rendered)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;
    use serde_json::json;

    fn feedback(status: &str, regen_count: i16, rendered: Option<Value>) -> Feedback {
        Feedback {
            id: Uuid::new_v4(),
            lesson_id: Uuid::new_v4(),
            notes: "今天讲判别式，前半节课听懂了，后面练习跟不上".to_owned(),
            knowledge_tags: vec!["偶有走神".to_owned(), "存在薄弱点".to_owned()],
            rendered,
            status: status.to_owned(),
            regen_count,
        }
    }

    #[test]
    fn template_keeps_notes_and_adds_suggestion() {
        let text = template::render(&feedback("draft", 0, None));
        assert!(text.contains("今天讲判别式"));
        assert!(text.contains("建议"));
    }

    #[test]
    fn generated_feedback_rejects_more_than_three_regenerations() {
        assert!(validate_state(&feedback("generated", 3, None)).is_err());
        assert!(validate_state(&feedback("generated", 2, None)).is_ok());
    }

    #[test]
    fn idempotency_requires_matching_key() {
        let rendered = json!({
            "text": "旧文本",
            "idempotencyKey": "key-1"
        });
        let stored = feedback("generated", 1, Some(rendered));
        assert_eq!(
            existing_generation(&stored),
            Some(("旧文本".to_owned(), "key-1".to_owned()))
        );
    }
}
