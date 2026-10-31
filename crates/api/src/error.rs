//! RFC 9457 Problem Details 响应与统一错误映射。

use axum::Json;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// 字段级校验错误，对应 Problem Details 的 `errors` 数组。
#[derive(Debug, Serialize)]
pub struct FieldError {
    /// 出错的字段名。
    pub field: String,
    /// 面向人的失败原因。
    pub reason: String,
}

/// API 统一错误。
///
/// 对外只暴露 `code` 与通用文案；`Internal` 会通过 `tracing::error!` 记录，
/// `detail` 不得包含敏感信息。
#[derive(Debug)]
pub enum ApiError {
    /// 资源不存在，或资源不属于当前租户。
    NotFound {
        /// 请求路径，写入 Problem Details 的 `instance`。
        instance: Option<String>,
    },
    /// 请求字段校验失败。
    Validation {
        /// 字段级失败原因。
        errors: Vec<FieldError>,
        /// 请求路径，写入 Problem Details 的 `instance`。
        instance: Option<String>,
    },
    /// 未预期的服务端错误。
    Internal {
        /// 面向人的错误说明，不包含内部堆栈或敏感信息。
        detail: String,
        /// 请求路径，写入 Problem Details 的 `instance`。
        instance: Option<String>,
    },
}

impl ApiError {
    /// 构造 404 错误。
    #[must_use]
    pub fn not_found(instance: Option<String>) -> Self {
        Self::NotFound { instance }
    }

    /// 构造 422 校验错误。
    #[must_use]
    pub fn validation(errors: Vec<FieldError>) -> Self {
        Self::Validation {
            errors,
            instance: None,
        }
    }

    /// 构造 500 内部错误。
    #[must_use]
    pub fn internal(detail: impl Into<String>) -> Self {
        Self::Internal {
            detail: detail.into(),
            instance: None,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, title, code, detail, instance, errors) = match self {
            Self::NotFound { instance } => (
                StatusCode::NOT_FOUND,
                "资源不存在",
                "NOT_FOUND",
                "请求的资源不存在".to_owned(),
                instance,
                Vec::new(),
            ),
            Self::Validation { errors, instance } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "请求校验失败",
                "VALIDATION_INVALID",
                "请求参数未通过校验".to_owned(),
                instance,
                errors,
            ),
            Self::Internal { detail, instance } => {
                tracing::error!(detail = %detail, "internal api error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "服务器内部错误",
                    "INTERNAL_ERROR",
                    detail,
                    instance,
                    Vec::new(),
                )
            }
        };

        let problem = ProblemDetails {
            type_uri: format!(
                "https://example.com/errors/{}",
                code.to_ascii_lowercase().replace('_', "-")
            ),
            title: title.to_owned(),
            status: status.as_u16(),
            detail,
            instance,
            code: code.to_owned(),
            errors,
        };

        let mut response = (status, Json(problem)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

#[derive(Debug, Serialize)]
struct ProblemDetails {
    #[serde(rename = "type")]
    type_uri: String,
    title: String,
    status: u16,
    detail: String,
    instance: Option<String>,
    code: String,
    errors: Vec<FieldError>,
}
