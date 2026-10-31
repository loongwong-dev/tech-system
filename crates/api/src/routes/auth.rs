//! 认证路由。

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use platform::{
    LoginError, LoginInput, RegistrationError, RegistrationInput, SESSION_TTL_SECONDS,
    SessionError, login as login_account, register as register_account, revoke_session,
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
            current_token: session_token(&headers),
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
    let token = session_token(&headers)
        .ok_or_else(|| ApiError::unauthorized("AUTH_SESSION_EXPIRED", "会话已失效"))?;
    revoke_session(state.db().admin(), &token)
        .await
        .map_err(map_session_error)?;

    let mut response = StatusCode::NO_CONTENT.into_response();
    clear_session_cookie(&mut response);
    Ok(response)
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix("sid=").map(str::to_owned))
}

fn set_session_cookie(response: &mut Response, token: &str) -> Result<(), ApiError> {
    let cookie = format!(
        "sid={token}; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age={SESSION_TTL_SECONDS}"
    );
    let value = HeaderValue::from_str(&cookie).map_err(|error| {
        tracing::error!(%error, "failed to build session cookie");
        ApiError::internal("会话 Cookie 生成失败")
    })?;
    response.headers_mut().insert(header::SET_COOKIE, value);
    Ok(())
}

fn clear_session_cookie(response: &mut Response) {
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("sid=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0"),
    );
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
