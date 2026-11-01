//! 反馈草稿读写路由。
//!
//! `GET /api/v1/lessons/{id}/feedbacks` 供课次录入页合并本地与服务端草稿；
//! `PUT /api/v1/feedbacks/{id}` 由客户端生成 id 幂等写入，防抖重试不会产生重复行。

use axum::Json;
use axum::extract::{Path, State};
use chrono::{DateTime, Utc};
use domain::{
    DRAFT_MAX_JSON_BYTES, DRAFT_MAX_NOTES_CHARS, DRAFT_MAX_TAG_CHARS, DRAFT_MAX_TAGS, DraftError,
    DraftInput, FeedbackDraft, list_lesson_drafts, save_draft,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

/// 保存草稿的请求体。
///
/// 四个可编辑字段为整体替换语义，缺一不可：字段缺失会被拒绝，而不是清空已有内容。
#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct SaveFeedbackRequest {
    /// 所属课次。
    lesson_id: String,
    /// 所属学员。
    student_id: String,
    /// 教师原文。
    notes: String,
    /// 打分对象。
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    ratings: Value,
    /// 知识点标签。
    knowledge_tags: Vec<String>,
    /// 作业对象。
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    homework: Value,
}

/// 反馈草稿响应。
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct FeedbackDraftResponse {
    /// 反馈主键，以服务端返回值为准。
    id: String,
    /// 所属课次。
    lesson_id: String,
    /// 所属学员。
    student_id: String,
    /// 教师原文。
    notes: String,
    /// 打分对象。
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    ratings: Value,
    /// 知识点标签。
    knowledge_tags: Vec<String>,
    /// 作业对象。
    #[schema(value_type = std::collections::HashMap<String, serde_json::Value>)]
    homework: Value,
    /// 反馈状态，取值为数据库状态值之一。
    status: String,
    /// 最近一次写入时间。
    updated_at: DateTime<Utc>,
}

/// 课次反馈列表。
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct FeedbackDraftListResponse {
    /// 该课次的全部反馈，按学员排序；没有反馈时为空数组。
    items: Vec<FeedbackDraftResponse>,
}

#[utoipa::path(
    get,
    path = "/api/v1/lessons/{id}/feedbacks",
    operation_id = "listLessonFeedbacks",
    tag = "feedbacks",
    security(("sid" = [])),
    params(("id" = String, Path, description = "课次 ID")),
    responses(
        (status = 200, description = "课次反馈列表", body = FeedbackDraftListResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "课次不存在", body = crate::error::ProblemDetails),
        (status = 422, description = "课次 ID 非法", body = crate::error::ProblemDetails)
    )
)]
/// 列出课次内全部反馈；课次没有反馈时返回空数组。
pub(crate) async fn index(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
) -> Result<Json<FeedbackDraftListResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let lesson_id = parse_uuid(&lesson_id, "id")?;
    let drafts = list_lesson_drafts(state.db().tenant(), tenant_id, lesson_id)
        .await
        .map_err(map_draft_error)?
        .ok_or_else(|| ApiError::not_found(None))?;
    Ok(Json(FeedbackDraftListResponse {
        items: drafts
            .into_iter()
            .map(FeedbackDraftResponse::from)
            .collect(),
    }))
}

#[utoipa::path(
    put,
    path = "/api/v1/feedbacks/{id}",
    operation_id = "saveFeedback",
    tag = "feedbacks",
    security(("sid" = [])),
    params(("id" = String, Path, description = "反馈 ID，客户端生成的 UUID")),
    request_body = SaveFeedbackRequest,
    responses(
        (status = 200, description = "草稿已保存", body = FeedbackDraftResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "课次、学员或反馈不存在（含路径 id 属于其他租户，不暴露存在性）", body = crate::error::ProblemDetails),
        (status = 409, description = "状态不允许编辑或反馈 ID 已被占用", body = crate::error::ProblemDetails),
        (status = 422, description = "字段非法或超限", body = crate::error::ProblemDetails)
    )
)]
/// 幂等保存草稿；`(lessonId, studentId)` 已有草稿时更新它并返回真实 id。
pub(crate) async fn save(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(feedback_id): Path<String>,
    Json(payload): Json<SaveFeedbackRequest>,
) -> Result<Json<FeedbackDraftResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let feedback_id = parse_uuid(&feedback_id, "id")?;
    let draft = save_draft(
        state.db().tenant(),
        tenant_id,
        feedback_id,
        &DraftInput {
            lesson_id: parse_uuid(&payload.lesson_id, "lessonId")?,
            student_id: parse_uuid(&payload.student_id, "studentId")?,
            notes: payload.notes,
            ratings: payload.ratings,
            knowledge_tags: payload.knowledge_tags,
            homework: payload.homework,
        },
    )
    .await
    .map_err(map_draft_error)?;
    Ok(Json(FeedbackDraftResponse::from(draft)))
}

impl From<FeedbackDraft> for FeedbackDraftResponse {
    fn from(draft: FeedbackDraft) -> Self {
        Self {
            id: draft.id.to_string(),
            lesson_id: draft.lesson_id.to_string(),
            student_id: draft.student_id.to_string(),
            notes: draft.notes,
            ratings: draft.ratings,
            knowledge_tags: draft.knowledge_tags,
            homework: draft.homework,
            status: draft.status,
            updated_at: draft.updated_at,
        }
    }
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: field.to_owned(),
            reason: "ID 格式无效".to_owned(),
        }])
    })
}

fn map_draft_error(error: DraftError) -> ApiError {
    match error {
        DraftError::LessonNotFound | DraftError::StudentNotFound | DraftError::NotFound => {
            ApiError::not_found(None)
        }
        // 重试后仍冲突：id 属于当前租户不可见的记录（通常来自其他租户），按契约返回 404 不暴露存在性。
        DraftError::IdConflict { source } => {
            tracing::warn!(%source, "feedback draft id conflict");
            ApiError::not_found(None)
        }
        DraftError::InvalidTransition => ApiError::conflict(
            "CONFLICT_INVALID_TRANSITION",
            "只有草稿状态允许编辑；已生成或已分享的反馈请重新生成或返回修改",
        ),
        DraftError::IdReused => ApiError::conflict(
            "CONFLICT_FEEDBACK_ID_REUSED",
            "该反馈 ID 已属于同一租户的其他学员，请重新生成 ID",
        ),
        DraftError::InvalidNotes => ApiError::validation(vec![FieldError {
            field: "notes".to_owned(),
            reason: format!("教师原文不能超过 {DRAFT_MAX_NOTES_CHARS} 个字符"),
        }]),
        DraftError::InvalidTags => ApiError::validation(vec![FieldError {
            field: "knowledgeTags".to_owned(),
            reason: format!(
                "标签最多 {DRAFT_MAX_TAGS} 个，单个标签 {DRAFT_MAX_TAG_CHARS} 个字符以内且不能为空"
            ),
        }]),
        DraftError::InvalidRatings => invalid_json_field("ratings"),
        DraftError::InvalidHomework => invalid_json_field("homework"),
        DraftError::Transaction { source } => {
            tracing::error!(%source, "feedback draft transaction error");
            ApiError::internal("反馈草稿操作失败")
        }
        DraftError::Database { source } => {
            tracing::error!(%source, "feedback draft database error");
            ApiError::internal("反馈草稿操作失败")
        }
    }
}

fn invalid_json_field(field: &str) -> ApiError {
    ApiError::validation(vec![FieldError {
        field: field.to_owned(),
        reason: format!("必须是 JSON 对象且不超过 {DRAFT_MAX_JSON_BYTES} 字节"),
    }])
}
