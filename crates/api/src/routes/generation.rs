//! 反馈生成与任务进度接口。
//!
//! 单份生成使用模板兜底并流式返回；批量生成只负责校验、入队与查询进度，
//! 实际生成由 worker 的 `feedback.generate_batch` handler 完成。

use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, HeaderName, StatusCode};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use domain::{GenerationError, generate_single, lesson_feedback_count};
use jobs::{Job, JobError, NewJob, enqueue, get as get_job};
use serde::Serialize;
use serde_json::{Value, json};
use tokio_stream::iter;
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

const IDEMPOTENCY_HEADER: HeaderName = HeaderName::from_static("idempotency-key");
const MAX_IDEMPOTENCY_KEY_CHARS: usize = 200;
const SSE_CHUNK_CHARS: usize = 16;

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct BatchJobResponse {
    job_id: String,
    status: String,
    total: i64,
    done: i64,
    failed: i64,
}

#[utoipa::path(
    post,
    path = "/api/v1/feedbacks/{id}/generations",
    operation_id = "generateFeedback",
    tag = "generation",
    security(("sid" = [])),
    params(("id" = String, Path, description = "反馈 ID")),
    responses(
        (status = 200, description = "SSE：delta 与 done 事件", body = String, content_type = "text/event-stream"),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "反馈不存在", body = crate::error::ProblemDetails),
        (status = 409, description = "状态或重生成次数不允许", body = crate::error::ProblemDetails)
    )
)]
/// 生成单份反馈，以 SSE 发送 `delta` 与 `done` 事件。
pub(crate) async fn generate_feedback(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(feedback_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let tenant_id = session.tenant_id()?;
    let feedback_id = parse_uuid(&feedback_id, "id")?;
    let idempotency_key = idempotency_key(&headers)?;

    let generated = generate_single(state.db().tenant(), tenant_id, feedback_id, idempotency_key)
        .await
        .map_err(map_generation_error)?;

    let mut events: Vec<Result<Event, Infallible>> = split_chunks(&generated.text)
        .into_iter()
        .map(|chunk| Ok(Event::default().event("delta").data(chunk)))
        .collect();
    let done = json!({
        "feedbackId": generated.feedback_id.to_string(),
        "text": generated.text,
        "generatedBy": generated.generated_by,
        "promptVersion": generated.prompt_version,
        "reused": generated.reused,
    });
    events.push(Ok(Event::default().event("done").data(done.to_string())));

    Ok(Sse::new(iter(events))
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/lessons/{id}/generations",
    operation_id = "generateLesson",
    tag = "generation",
    security(("sid" = [])),
    params(("id" = String, Path, description = "课次 ID")),
    responses(
        (status = 202, description = "批量任务已入队", body = BatchJobResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "课次不存在", body = crate::error::ProblemDetails),
        (status = 422, description = "课次没有反馈", body = crate::error::ProblemDetails)
    )
)]
/// 校验课次并创建批量生成任务。
pub(crate) async fn generate_lesson(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(lesson_id): Path<String>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let tenant_id = session.tenant_id()?;
    let lesson_id = parse_uuid(&lesson_id, "id")?;
    let idempotency_key = idempotency_key(&headers)?;

    let total = lesson_feedback_count(state.db().tenant(), tenant_id, lesson_id)
        .await
        .map_err(map_generation_error)?
        .ok_or_else(|| ApiError::not_found(None))?;
    if total == 0 {
        return Err(ApiError::validation(vec![FieldError {
            field: "lessonId".to_owned(),
            reason: "课次没有反馈".to_owned(),
        }]));
    }

    let mut new_job = NewJob::new(
        tenant_id,
        "feedback.generate_batch",
        json!({ "lessonId": lesson_id.to_string(), "total": total }),
    );
    if let Some(key) = idempotency_key {
        new_job = new_job.with_dedupe_key(format!("lesson:{lesson_id}:{key}"));
    }

    let job = enqueue(state.db().admin().pool(), new_job)
        .await
        .map_err(map_job_error)?;
    let response = BatchJobResponse::from_job(&job, total);

    Ok((StatusCode::ACCEPTED, Json(response)).into_response())
}

#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}",
    operation_id = "getJobProgress",
    tag = "generation",
    security(("sid" = [])),
    params(("id" = String, Path, description = "任务 ID")),
    responses(
        (status = 200, description = "任务状态与进度", body = BatchJobResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "任务不存在", body = crate::error::ProblemDetails)
    )
)]
/// 查询当前租户的任务进度。
pub(crate) async fn job_progress(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<BatchJobResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let job_id = parse_uuid(&job_id, "id")?;
    let job = get_job(state.db().admin().pool(), tenant_id, job_id)
        .await
        .map_err(map_job_error)?
        .ok_or_else(|| ApiError::not_found(None))?;
    let total = job_total(&job);

    Ok(Json(BatchJobResponse::from_job(&job, total)))
}

impl BatchJobResponse {
    fn from_job(job: &Job, fallback_total: i64) -> Self {
        let total = job_total(job).max(fallback_total);
        let done = if job.status == jobs::STATUS_SUCCEEDED {
            total
        } else {
            0
        };
        let failed = if job.status == jobs::STATUS_DEAD {
            total
        } else {
            0
        };
        Self {
            job_id: job.id.to_string(),
            status: job.status.clone(),
            total,
            done,
            failed,
        }
    }
}

fn job_total(job: &Job) -> i64 {
    job.payload
        .get("total")
        .and_then(Value::as_i64)
        .unwrap_or(0)
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: field.to_owned(),
            reason: "ID 格式无效".to_owned(),
        }])
    })
}

fn idempotency_key(headers: &HeaderMap) -> Result<Option<&str>, ApiError> {
    let Some(value) = headers.get(&IDEMPOTENCY_HEADER) else {
        return Ok(None);
    };
    let key = value.to_str().map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: "Idempotency-Key".to_owned(),
            reason: "必须为 ASCII 字符串".to_owned(),
        }])
    })?;
    if key.trim().is_empty() {
        return Err(ApiError::validation(vec![FieldError {
            field: "Idempotency-Key".to_owned(),
            reason: "不能为空".to_owned(),
        }]));
    }
    if key.chars().count() > MAX_IDEMPOTENCY_KEY_CHARS {
        return Err(ApiError::validation(vec![FieldError {
            field: "Idempotency-Key".to_owned(),
            reason: format!("不能超过 {MAX_IDEMPOTENCY_KEY_CHARS} 个字符"),
        }]));
    }
    Ok(Some(key))
}

fn split_chunks(text: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    characters
        .chunks(SSE_CHUNK_CHARS)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn map_generation_error(error: GenerationError) -> ApiError {
    match error {
        GenerationError::NotFound | GenerationError::LessonNotFound => ApiError::not_found(None),
        GenerationError::LessonEmpty => ApiError::validation(vec![FieldError {
            field: "lessonId".to_owned(),
            reason: "课次没有反馈".to_owned(),
        }]),
        GenerationError::RegenerationLimit => {
            ApiError::conflict("CONFLICT_REGENERATION_LIMIT", "重新生成次数已达上限")
        }
        GenerationError::InvalidState => {
            ApiError::conflict("CONFLICT_GENERATION_STATE", "当前状态不允许生成")
        }
        GenerationError::Transaction { source } => {
            tracing::error!(%source, "generation transaction error");
            ApiError::internal("反馈生成失败")
        }
        GenerationError::Database { source } => {
            tracing::error!(%source, "generation database error");
            ApiError::internal("反馈生成失败")
        }
    }
}

fn map_job_error(error: JobError) -> ApiError {
    match error {
        JobError::InvalidInput { reason } => ApiError::validation(vec![FieldError {
            field: "job".to_owned(),
            reason: reason.to_owned(),
        }]),
        JobError::Config { reason } => {
            tracing::error!(%reason, "job config error");
            ApiError::internal("任务配置无效")
        }
        JobError::Database { source } => {
            tracing::error!(%source, "job database error");
            ApiError::internal("任务队列操作失败")
        }
    }
}
