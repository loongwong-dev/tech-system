//! 课次列表与创建路由。

use axum::Json;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, NaiveDate, Utc};
use domain::{
    LESSON_DEFAULT_LIMIT, LESSON_MAX_LIMIT, LESSON_MAX_TOPIC_CHARS, Lesson, LessonCursor,
    LessonError, LessonFilter, LessonInput, create_lesson, list_lessons,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

#[derive(Debug, Deserialize)]
pub(crate) struct ListLessonsQuery {
    from: Option<String>,
    to: Option<String>,
    limit: Option<String>,
    cursor: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct CreateLessonRequest {
    class_id: String,
    #[schema(format = Date)]
    lesson_date: String,
    topic: Option<String>,
}

/// 课次状态，与数据库 CHECK 约束一致。
#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LessonStatus {
    /// 草稿。
    Draft,
    /// 已完成。
    Done,
}

impl TryFrom<&str> for LessonStatus {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "draft" => Ok(Self::Draft),
            "done" => Ok(Self::Done),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct LessonResponse {
    id: String,
    class_id: String,
    lesson_date: NaiveDate,
    topic: Option<String>,
    status: LessonStatus,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct LessonListResponse {
    items: Vec<LessonResponse>,
    next_cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/v1/lessons",
    operation_id = "listLessons",
    tag = "lessons",
    security(("sid" = [])),
    params(
        ("from" = Option<String>, Query, description = "起始日期（含），YYYY-MM-DD"),
        ("to" = Option<String>, Query, description = "结束日期（含），YYYY-MM-DD"),
        ("limit" = Option<i64>, Query, description = "每页数量，默认 20，上限 100"),
        ("cursor" = Option<String>, Query, description = "上一页返回的 nextCursor")
    ),
    responses(
        (status = 200, description = "课次列表", body = LessonListResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 422, description = "日期、分页或游标非法", body = crate::error::ProblemDetails)
    )
)]
/// 按日期范围列出当前租户课次，使用游标分页。
pub(crate) async fn index(
    session: SessionAuth,
    State(state): State<AppState>,
    Query(query): Query<ListLessonsQuery>,
) -> Result<Json<LessonListResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let from = query
        .from
        .as_deref()
        .map(|raw| parse_date(raw, "from"))
        .transpose()?;
    let to = query
        .to
        .as_deref()
        .map(|raw| parse_date(raw, "to"))
        .transpose()?;
    let limit = parse_limit(query.limit.as_deref())?;
    let cursor = query.cursor.as_deref().map(decode_cursor).transpose()?;

    let page = list_lessons(
        state.db().tenant(),
        tenant_id,
        LessonFilter {
            from,
            to,
            limit,
            cursor,
        },
    )
    .await
    .map_err(map_lesson_error)?;

    let items = page
        .items
        .into_iter()
        .map(LessonResponse::try_from)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Json(LessonListResponse {
        items,
        next_cursor: page.next_cursor.map(encode_cursor),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/lessons",
    operation_id = "createLesson",
    tag = "lessons",
    security(("sid" = [])),
    request_body = CreateLessonRequest,
    responses(
        (status = 201, description = "课次已创建", body = LessonResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "班级不存在", body = crate::error::ProblemDetails),
        (status = 422, description = "日期或主题非法", body = crate::error::ProblemDetails)
    )
)]
/// 创建课次；班级必须属于当前租户。
pub(crate) async fn create(
    session: SessionAuth,
    State(state): State<AppState>,
    Json(payload): Json<CreateLessonRequest>,
) -> Result<Response, ApiError> {
    let tenant_id = session.tenant_id()?;
    let class_id = parse_uuid(&payload.class_id, "classId")?;
    let lesson_date = parse_date(&payload.lesson_date, "lessonDate")?;

    let lesson = create_lesson(
        state.db().tenant(),
        tenant_id,
        LessonInput {
            class_id,
            lesson_date,
            topic: payload.topic.as_deref(),
        },
    )
    .await
    .map_err(map_lesson_error)?;

    let response = LessonResponse::try_from(lesson)?;
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

impl TryFrom<Lesson> for LessonResponse {
    type Error = ApiError;

    fn try_from(lesson: Lesson) -> Result<Self, Self::Error> {
        let status = LessonStatus::try_from(lesson.status.as_str())
            .map_err(|()| ApiError::internal("课次状态无效"))?;
        Ok(Self {
            id: lesson.id.to_string(),
            class_id: lesson.class_id.to_string(),
            lesson_date: lesson.lesson_date,
            topic: lesson.topic,
            status,
            created_at: lesson.created_at,
        })
    }
}

fn parse_date(raw: &str, field: &str) -> Result<NaiveDate, ApiError> {
    NaiveDate::parse_from_str(raw, "%Y-%m-%d").map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: field.to_owned(),
            reason: "日期格式必须为 YYYY-MM-DD".to_owned(),
        }])
    })
}

fn parse_limit(raw: Option<&str>) -> Result<i64, ApiError> {
    let Some(raw) = raw else {
        return Ok(LESSON_DEFAULT_LIMIT);
    };
    raw.parse::<i64>().map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: "limit".to_owned(),
            reason: "必须是整数".to_owned(),
        }])
    })
}

fn parse_uuid(raw: &str, field: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: field.to_owned(),
            reason: "ID 格式无效".to_owned(),
        }])
    })
}

fn encode_cursor(cursor: LessonCursor) -> String {
    let raw = format!("{}|{}", cursor.lesson_date.format("%Y-%m-%d"), cursor.id);
    URL_SAFE_NO_PAD.encode(raw)
}

fn decode_cursor(raw: &str) -> Result<LessonCursor, ApiError> {
    let bytes = URL_SAFE_NO_PAD.decode(raw).map_err(|_| invalid_cursor())?;
    let text = String::from_utf8(bytes).map_err(|_| invalid_cursor())?;
    let (date, id) = text.split_once('|').ok_or_else(invalid_cursor)?;
    let lesson_date =
        chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|_| invalid_cursor())?;
    let id = Uuid::parse_str(id).map_err(|_| invalid_cursor())?;
    Ok(LessonCursor { lesson_date, id })
}

fn invalid_cursor() -> ApiError {
    ApiError::validation(vec![FieldError {
        field: "cursor".to_owned(),
        reason: "游标无效".to_owned(),
    }])
}

fn map_lesson_error(error: LessonError) -> ApiError {
    match error {
        LessonError::ClassNotFound => ApiError::not_found(None),
        LessonError::InvalidTopic => ApiError::validation(vec![FieldError {
            field: "topic".to_owned(),
            reason: format!("主题不能超过 {LESSON_MAX_TOPIC_CHARS} 个字符"),
        }]),
        LessonError::InvalidLimit => ApiError::validation(vec![FieldError {
            field: "limit".to_owned(),
            reason: format!("必须在 1 到 {LESSON_MAX_LIMIT} 之间"),
        }]),
        LessonError::InvalidDateRange => ApiError::validation(vec![FieldError {
            field: "from".to_owned(),
            reason: "起始日期不能晚于结束日期".to_owned(),
        }]),
        LessonError::Transaction { source } => {
            tracing::error!(%source, "lesson transaction error");
            ApiError::internal("课次操作失败")
        }
        LessonError::Database { source } => {
            tracing::error!(%source, "lesson database error");
            ApiError::internal("课次操作失败")
        }
    }
}
