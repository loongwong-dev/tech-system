//! 班级管理路由。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use domain::{
    Class, ClassError, ClassInput, create_class, delete_class, list_classes, update_class,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateClassRequest {
    name: String,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateClassRequest {
    name: String,
    subject: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClassResponse {
    id: String,
    name: String,
    subject: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClassListResponse {
    items: Vec<ClassResponse>,
}

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
