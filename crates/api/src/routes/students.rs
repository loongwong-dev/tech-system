//! 学员管理路由。

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use domain::{
    Student, StudentError, StudentInput, archive_student, create_student, delete_student,
    list_students, update_student,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session::SessionAuth;
use crate::{ApiError, AppState, FieldError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateStudentRequest {
    name: String,
    grade: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateStudentRequest {
    name: String,
    grade: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DeleteStudentRequest {
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StudentResponse {
    id: String,
    name: String,
    grade: Option<String>,
    subject: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StudentListResponse {
    items: Vec<StudentResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeletedStudentResponse {
    id: String,
    feedback_count: i64,
}

/// 列出当前租户未归档学员。
pub(crate) async fn index(
    session: SessionAuth,
    State(state): State<AppState>,
) -> Result<Json<StudentListResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let students = list_students(state.db().tenant(), tenant_id)
        .await
        .map_err(map_student_error)?;
    Ok(Json(StudentListResponse {
        items: students.into_iter().map(StudentResponse::from).collect(),
    }))
}

/// 创建学员。
pub(crate) async fn create(
    session: SessionAuth,
    State(state): State<AppState>,
    Json(payload): Json<CreateStudentRequest>,
) -> Result<Response, ApiError> {
    let tenant_id = session.tenant_id()?;
    let student = create_student(
        state.db().tenant(),
        tenant_id,
        StudentInput {
            name: &payload.name,
            grade: payload.grade.as_deref(),
            subject: payload.subject.as_deref(),
        },
    )
    .await
    .map_err(map_student_error)?;
    Ok((StatusCode::CREATED, Json(StudentResponse::from(student))).into_response())
}

/// 更新学员。
pub(crate) async fn update(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(student_id): Path<String>,
    Json(payload): Json<UpdateStudentRequest>,
) -> Result<Json<StudentResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let student_id = parse_uuid(&student_id, "id")?;
    let student = update_student(
        state.db().tenant(),
        tenant_id,
        student_id,
        StudentInput {
            name: &payload.name,
            grade: payload.grade.as_deref(),
            subject: payload.subject.as_deref(),
        },
    )
    .await
    .map_err(map_student_error)?;
    Ok(Json(StudentResponse::from(student)))
}

/// 归档学员。
pub(crate) async fn archive(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(student_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let tenant_id = session.tenant_id()?;
    let student_id = parse_uuid(&student_id, "id")?;
    archive_student(state.db().tenant(), tenant_id, student_id)
        .await
        .map_err(map_student_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// 删除学员；请求体姓名必须与当前姓名一致。
pub(crate) async fn delete(
    session: SessionAuth,
    State(state): State<AppState>,
    Path(student_id): Path<String>,
    Json(payload): Json<DeleteStudentRequest>,
) -> Result<Json<DeletedStudentResponse>, ApiError> {
    let tenant_id = session.tenant_id()?;
    let student_id = parse_uuid(&student_id, "id")?;
    let deleted = delete_student(
        state.db().tenant(),
        tenant_id,
        session.context.user_id,
        student_id,
        &payload.name,
    )
    .await
    .map_err(map_student_error)?;
    Ok(Json(DeletedStudentResponse {
        id: deleted.id.to_string(),
        feedback_count: deleted.feedback_count,
    }))
}

impl From<Student> for StudentResponse {
    fn from(student: Student) -> Self {
        Self {
            id: student.id.to_string(),
            name: student.name,
            grade: student.grade,
            subject: student.subject,
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

fn map_student_error(error: StudentError) -> ApiError {
    match error {
        StudentError::NotFound => ApiError::not_found(None),
        StudentError::InvalidName => ApiError::validation(vec![FieldError {
            field: "name".to_owned(),
            reason: "姓名不能为空且不能超过 50 个字符".to_owned(),
        }]),
        StudentError::NameMismatch => ApiError::validation_named(
            "VALIDATION_NAME_MISMATCH",
            "学员姓名不匹配",
            vec![FieldError {
                field: "name".to_owned(),
                reason: "请提交与当前学员一致的姓名".to_owned(),
            }],
        ),
        StudentError::Transaction { source } => {
            tracing::error!(%source, "student transaction error");
            ApiError::internal("学员操作失败")
        }
        StudentError::Database { source } => {
            tracing::error!(%source, "student database error");
            ApiError::internal("学员操作失败")
        }
    }
}
