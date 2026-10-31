//! 认证路由。

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use platform::{
    RegistrationError, RegistrationInput, SESSION_TTL_SECONDS, register as register_account,
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, AppState, FieldError};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterRequest {
    email: String,
    password: String,
    invite_code: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterResponse {
    user_id: String,
    tenant_id: String,
}

/// 使用邀请码注册个人租户，并建立 30 天会话。
pub(crate) async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Response, ApiError> {
    let account = register_account(
        state.db().admin(),
        RegistrationInput {
            email: payload.email,
            password: payload.password,
            invite_code: payload.invite_code,
        },
    )
    .await
    .map_err(map_registration_error)?;

    let cookie = format!(
        "sid={}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={SESSION_TTL_SECONDS}",
        account.session_token.as_str()
    );
    let cookie = HeaderValue::from_str(&cookie).map_err(|error| {
        tracing::error!(%error, "failed to build session cookie");
        ApiError::internal("注册失败")
    })?;

    let mut response = (
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: account.user_id.to_string(),
            tenant_id: account.tenant_id.to_string(),
        }),
    )
        .into_response();
    response.headers_mut().insert(header::SET_COOKIE, cookie);
    Ok(response)
}

fn map_registration_error(error: RegistrationError) -> ApiError {
    match error {
        RegistrationError::InvalidEmail => ApiError::validation(vec![FieldError {
            field: "email".to_owned(),
            reason: "邮箱格式无效".to_owned(),
        }]),
        RegistrationError::WeakPassword => ApiError::validation(vec![FieldError {
            field: "password".to_owned(),
            reason: "密码不符合要求".to_owned(),
        }]),
        RegistrationError::InvalidInviteCode => {
            ApiError::conflict("CONFLICT_INVITE_CODE_INVALID", "邀请码无效")
        }
        RegistrationError::EmailExists => ApiError::conflict("CONFLICT_EMAIL_EXISTS", "邮箱已注册"),
        RegistrationError::Internal => ApiError::internal("注册失败"),
        RegistrationError::Database { source } => {
            tracing::error!(%source, "registration database error");
            ApiError::internal("注册失败")
        }
    }
}
