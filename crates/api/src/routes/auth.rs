//! 认证路由。

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use platform::{
    LoginError, LoginInput, RegistrationError, RegistrationInput, SessionError, SwitchTenantError,
    login as login_account, register as register_account, revoke_session,
    switch_tenant as switch_tenant_account,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::session::{clear_session_cookie, set_session_cookie, token_from_headers};
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LoginResponse {
    user_id: String,
    tenant_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TenantSwitchRequest {
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

    let mut response = (
        StatusCode::CREATED,
        Json(RegisterResponse {
            user_id: account.user_id.to_string(),
            tenant_id: account.tenant_id.to_string(),
        }),
    )
        .into_response();
    set_session_cookie(&mut response, account.session_token.as_str())?;
    Ok(response)
}

/// 邮箱密码登录；请求携带旧 `sid` 时轮换旧会话。
pub(crate) async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Response, ApiError> {
    let result = login_account(
        state.db().admin(),
        LoginInput {
            email: payload.email,
            password: payload.password,
            current_token: token_from_headers(&headers),
        },
    )
    .await
    .map_err(map_login_error)?;

    let mut response = (
        StatusCode::OK,
        Json(LoginResponse {
            user_id: result.user_id.to_string(),
            tenant_id: result.tenant_id.map(|tenant_id| tenant_id.to_string()),
        }),
    )
        .into_response();
    set_session_cookie(&mut response, result.session_token.as_str())?;
    Ok(response)
}

/// 吊销当前会话并清除 Cookie。
pub(crate) async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let token = token_from_headers(&headers)
        .ok_or_else(|| ApiError::unauthorized("AUTH_SESSION_EXPIRED", "会话已失效"))?;
    revoke_session(state.db().admin(), &token)
        .await
        .map_err(map_session_error)?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    clear_session_cookie(&mut response);
    Ok(response)
}

/// 校验 membership 后切换租户并轮换会话。
pub(crate) async fn switch_tenant(
    session: crate::session::SessionAuth,
    State(state): State<AppState>,
    Json(payload): Json<TenantSwitchRequest>,
) -> Result<Response, ApiError> {
    let tenant_id = Uuid::parse_str(&payload.tenant_id).map_err(|_| {
        ApiError::validation(vec![FieldError {
            field: "tenantId".to_owned(),
            reason: "租户 ID 格式无效".to_owned(),
        }])
    })?;
    let token = switch_tenant_account(state.db().admin(), &session.token, tenant_id)
        .await
        .map_err(map_switch_error)?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    set_session_cookie(&mut response, token.as_str())?;
    Ok(response)
}

fn map_switch_error(error: SwitchTenantError) -> ApiError {
    match error {
        SwitchTenantError::ForbiddenTenant => {
            ApiError::forbidden("FORBIDDEN_TENANT", "无权切换到该租户")
        }
        SwitchTenantError::SessionExpired => {
            ApiError::unauthorized("AUTH_SESSION_EXPIRED", "会话已失效")
        }
        SwitchTenantError::Database { source } => {
            tracing::error!(%source, "tenant switch database error");
            ApiError::internal("租户切换失败")
        }
    }
}

fn map_login_error(error: LoginError) -> ApiError {
    match error {
        LoginError::InvalidCredentials => {
            ApiError::unauthorized("AUTH_INVALID_CREDENTIALS", "邮箱或密码错误")
        }
        LoginError::Internal => ApiError::internal("登录失败"),
        LoginError::Database { source } => {
            tracing::error!(%source, "login database error");
            ApiError::internal("登录失败")
        }
    }
}

fn map_session_error(error: SessionError) -> ApiError {
    match error {
        SessionError::Expired => ApiError::unauthorized("AUTH_SESSION_EXPIRED", "会话已失效"),
        SessionError::Database { source } => {
            tracing::error!(%source, "session database error");
            ApiError::internal("会话操作失败")
        }
    }
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
