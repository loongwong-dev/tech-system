//! 班级管理路由。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use domain::{
    Class, ClassError, ClassInput, archive_class, create_class, delete_class, list_classes,
    update_class,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct CreateClassRequest {
    name: String,
    subject: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct UpdateClassRequest {
    name: String,
    subject: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct ClassResponse {
    id: String,
    name: String,
    subject: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(rename_all = "camelCase")]
pub(crate) struct ClassListResponse {
    items: Vec<ClassResponse>,
}

#[utoipa::path(
    get,
    path = "/api/v1/classes",
    operation_id = "listClasses",
    tag = "classes",
    security(("sid" = [])),
    responses(
        (status = 200, description = "班级列表", body = ClassListResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails)
    )
)]
/// 列出当前租户全部班级。
pub(crate) async fn index(
    session: SessionAuth,
    State(state): State<AppState>,
) -> Result<Json<ClassListResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let classes = list_classes(state.db().tenant(), tenant_id)
        .await
        .map_err(map_class_error)?;
    Ok(Json(ClassListResponse {
        items: classes.into_iter().map(ClassResponse::from).collect(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/classes",
    operation_id = "createClass",
    tag = "classes",
    security(("sid" = [])),
    request_body = CreateClassRequest,
    responses(
        (status = 201, description = "班级已创建", body = ClassResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 422, description = "名称无效", body = crate::error::ProblemDetails)
    )
)]
/// 创建班级。
pub(crate) async fn create(
    session: SessionAuth,
    State(state): State<AppState>,
    Json(payload): Json<CreateClassRequest>,
) -> Result<Response, ApiError> {
    let tenant_id = session.tenant_id()?;
    let class = create_class(
        state.db().tenant(),
        tenant_id,
        ClassInput {
            name: &payload.name,
            subject: payload.subject.as_deref(),
        },
    )
    .await
    .map_err(map_class_error)?;
    Ok((StatusCode::CREATED, Json(ClassResponse::from(class))).into_response())
}

#[utoipa::path(
    put,
    path = "/api/v1/classes/{id}",
    operation_id = "updateClass",
    tag = "classes",
    security(("sid" = [])),
    params(("id" = String, Path, description = "班级 ID")),
    request_body = UpdateClassRequest,
    responses(
        (status = 200, description = "班级已更新", body = ClassResponse),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "班级不存在", body = crate::error::ProblemDetails),
        (status = 422, description = "名称无效", body = crate::error::ProblemDetails)
    )
)]
/// 更新班级。
pub(crate) async fn update(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(class_id): Path<String>,
    Json(payload): Json<UpdateClassRequest>,
) -> Result<Json<ClassResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let class_id = parse_uuid(&class_id, "id")?;
    let class = update_class(
        state.db().tenant(),
        tenant_id,
        class_id,
        ClassInput {
            name: &payload.name,
            subject: payload.subject.as_deref(),
        },
    )
    .await
    .map_err(map_class_error)?;
    Ok(Json(ClassResponse::from(class)))
}

#[utoipa::path(
    post,
    path = "/api/v1/classes/{id}/archive",
    operation_id = "archiveClass",
    tag = "classes",
    security(("sid" = [])),
    params(("id" = String, Path, description = "班级 ID")),
    responses(
        (status = 204, description = "班级已归档"),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "班级不存在", body = crate::error::ProblemDetails)
    )
)]
/// 归档班级；归档后不再出现在默认列表。
pub(crate) async fn archive(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(class_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let tenant_id = session.tenant_id()?;
    let class_id = parse_uuid(&class_id, "id")?;
    archive_class(state.db().tenant(), tenant_id, class_id)
        .await
        .map_err(map_class_error)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    delete,
    path = "/api/v1/classes/{id}",
    operation_id = "deleteClass",
    tag = "classes",
    security(("sid" = [])),
    params(("id" = String, Path, description = "班级 ID")),
    responses(
        (status = 204, description = "班级已删除"),
        (status = 401, description = "未认证", body = crate::error::ProblemDetails),
        (status = 404, description = "班级不存在", body = crate::error::ProblemDetails),
        (status = 409, description = "班级仍有课次", body = crate::error::ProblemDetails)
    )
)]
/// 删除没有课次的班级。
pub(crate) async fn delete(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(class_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let tenant_id = session.tenant_id()?;
    let class_id = parse_uuid(&class_id, "id")?;
    delete_class(state.db().tenant(), tenant_id, class_id)
        .await
        .map_err(map_class_error)?;
    Ok(StatusCode::NO_CONTENT)
}

impl From<Class> for ClassResponse {
    fn from(class: Class) -> Self {
        Self {
            id: class.id.to_string(),
            name: class.name,
            subject: class.subject,
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

fn map_class_error(error: ClassError) -> ApiError {
    match error {
        ClassError::NotFound => ApiError::not_found(None),
        ClassError::InvalidName => ApiError::validation(vec![FieldError {
            field: "name".to_owned(),
            reason: "名称不能为空且不能超过 50 个字符".to_owned(),
        }]),
        ClassError::HasLessons => {
            ApiError::conflict("CONFLICT_CLASS_HAS_LESSONS", "班级仍有课次，不能删除")
        }
        ClassError::Transaction { source } => {
            tracing::error!(%source, "class transaction error");
            ApiError::internal("班级操作失败")
        }
        ClassError::Database { source } => {
            tracing::error!(%source, "class database error");
            ApiError::internal("班级操作失败")
        }
    }
}
