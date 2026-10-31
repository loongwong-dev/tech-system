//! 会话 Cookie 工具与认证 extractor。

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderMap, HeaderValue, header};
use axum::response::Response;
use platform::{SESSION_TTL_SECONDS, SessionContext, SessionError, validate_session};
use uuid::Uuid;

use crate::{ApiError, AppState};

/// 已认证会话；租户上下文只来自服务端会话。
pub(crate) struct SessionAuth {
    pub(crate) context: SessionContext,
    pub(crate) token: String,
}

impl SessionAuth {
    /// 返回当前会话的租户；未选择租户时返回 401。
    pub(crate) fn tenant_id(&self) -> Result<Uuid, ApiError> {
        self.context
            .tenant_id
            .ok_or_else(|| ApiError::unauthorized("AUTH_TENANT_REQUIRED", "请先选择租户"))
    }
}

impl FromRequestParts<AppState> for SessionAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = token_from_headers(&parts.headers)
            .ok_or_else(|| ApiError::unauthorized("AUTH_SESSION_EXPIRED", "会话已失效"))?;
        let context = validate_session(state.db().admin(), &token)
            .await
            .map_err(map_session_error)?;
        Ok(Self { context, token })
    }
}

pub(crate) fn token_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .find_map(|part| part.trim().strip_prefix("sid=").map(str::to_owned))
}

pub(crate) fn set_session_cookie(response: &mut Response, token: &str) -> Result<(), ApiError> {
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

pub(crate) fn clear_session_cookie(response: &mut Response) {
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("sid=; HttpOnly; Secure; SameSite=Lax; Path=/; Max-Age=0"),
    );
}

fn map_session_error(error: SessionError) -> ApiError {
    match error {
        SessionError::Expired => ApiError::unauthorized("AUTH_SESSION_EXPIRED", "会话已失效"),
        SessionError::Database { source } => {
            tracing::error!(%source, "session database error");
            ApiError::internal("会话校验失败")
        }
    }
}
