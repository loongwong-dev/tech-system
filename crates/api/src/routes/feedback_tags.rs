//! 反馈标签目录与学员标签继承路由。
//!
//! 目录是静态数据，只要求已认证；继承查询按租户进行，跨租户学员返回 404。

use axum::Json;
use axum::extract::{Path, State};
use domain::{
    CatalogDimension, CatalogTag, InheritedTags, TagQueryError, TagTone, latest_student_tags,
    list_catalog,
};
use serde::Serialize;
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

/// 标签语气权重响应。
#[derive(Debug, Clone, Copy, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FeedbackTagTone {
    /// 正面。
    Positive,
    /// 中性。
    Neutral,
    /// 待改进。
    Negative,
}

impl From<TagTone> for FeedbackTagTone {
    fn from(tone: TagTone) -> Self {
        match tone {
            TagTone::Positive => Self::Positive,
            TagTone::Neutral => Self::Neutral,
            TagTone::Negative => Self::Negative,
        }
    }
}

/// 目录中的一个标签。
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct FeedbackTagResponse {
    /// 标签名，写入反馈时原样提交。
    name: String,
    /// 语气权重。
    tone: FeedbackTagTone,
}

/// 目录中的一个维度。
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct FeedbackTagDimensionResponse {
    /// 维度名。
    name: String,
    /// 该维度下的标签，顺序即选择器顺序。
    tags: Vec<FeedbackTagResponse>,
}

/// 内置标签目录。
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct FeedbackTagCatalogResponse {
    /// 五个维度及其标签。
    dimensions: Vec<FeedbackTagDimensionResponse>,
}

/// 学员上次使用的标签组合。
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct InheritedTagsResponse {
    /// 标签组合，没有历史时为空数组。
    tags: Vec<String>,
    /// 由标签推导出的语气权重；空组合为 `neutral`。
    tone: FeedbackTagTone,
}

#[utoipa::path(
    get,
    path = "/api/v1/feedback-tags",
    operation_id = "listFeedbackTags",
    tag = "feedbacks",
    security(("sid" = [])),
    responses(
        (status = 200, description = "内置标签目录", body = FeedbackTagCatalogResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails)
    )
)]
/// 列出内置标签目录；数据来自代码常量，不需要租户上下文。
pub(crate) async fn catalog(
    _session: SessionAuth,
) -> Result<Json<FeedbackTagCatalogResponse>, ApiError> {
    Ok(Json(FeedbackTagCatalogResponse::from(list_catalog())))
}

#[utoipa::path(
    get,
    path = "/api/v1/students/{id}/feedback-tags",
    operation_id = "getStudentFeedbackTags",
    tag = "feedbacks",
    security(("sid" = [])),
    params(("id" = String, Path, description = "学员 ID")),
    responses(
        (status = 200, description = "学员最近一次使用的标签组合", body = InheritedTagsResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "学员不存在", body = crate::error::ProblemDetails),
        (status = 422, description = "学员 ID 非法", body = crate::error::ProblemDetails)
    )
)]
/// 返回学员最近一次使用的标签组合，供录入页继承默认值；没有历史时返回空组合。
pub(crate) async fn inherited(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(student_id): Path<String>,
) -> Result<Json<InheritedTagsResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let student_id = parse_uuid(&student_id)?;
    let inherited = latest_student_tags(state.db().tenant(), tenant_id, student_id)
        .await
        .map_err(map_tag_error)?
        .ok_or_else(|| ApiError::not_found(None))?;
    Ok(Json(InheritedTagsResponse::from(inherited)))
}

impl From<CatalogDimension> for FeedbackTagDimensionResponse {
    fn from(dimension: CatalogDimension) -> Self {
        Self {
            name: dimension.name.to_owned(),
            tags: dimension
                .tags
                .into_iter()
                .map(FeedbackTagResponse::from)
                .collect(),
        }
    }
}

impl From<CatalogTag> for FeedbackTagResponse {
    fn from(tag: CatalogTag) -> Self {
        Self {
            name: tag.name.to_owned(),
            tone: FeedbackTagTone::from(tag.tone),
        }
    }
}

impl From<Vec<CatalogDimension>> for FeedbackTagCatalogResponse {
    fn from(dimensions: Vec<CatalogDimension>) -> Self {
        Self {
            dimensions: dimensions
                .into_iter()
                .map(FeedbackTagDimensionResponse::from)
                .collect(),
        }
    }
}

impl From<InheritedTags> for InheritedTagsResponse {
    fn from(inherited: InheritedTags) -> Self {
        Self {
            tags: inherited.tags,
            tone: FeedbackTagTone::from(inherited.tone),
        }
    }
}

fn parse_uuid(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: "id".to_owned(),
            reason: "ID 格式无效".to_owned(),
        }])
    })
}

fn map_tag_error(error: TagQueryError) -> ApiError {
    match error {
        TagQueryError::Transaction { source } => {
            tracing::error!(%source, "feedback tag transaction error");
            ApiError::internal("标签查询失败")
        }
        TagQueryError::Database { source } => {
            tracing::error!(%source, "feedback tag database error");
            ApiError::internal("标签查询失败")
        }
    }
}
