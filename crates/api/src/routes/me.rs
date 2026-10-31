//! 当前账号与租户信息。

use axum::Json;
use axum::extract::State;
use platform::{MeError, me as me_account};
use serde::Serialize;

use crate::session::SessionAuth;
use crate::{ApiError, AppState};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MeResponse {
    user_id: String,
    email: String,
    tenant_id: String,
    tenant_name: String,
    role: String,
    quota: QuotaResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuotaResponse {
    limit: i64,
    used: i64,
    remaining: i64,
    reset_at: String,
}

/// 返回会话绑定的用户、租户、角色与配额。
pub(crate) async fn me(
    session: SessionAuth,
    State(state): State<AppState>,
) -> Result<Json<MeResponse>, ApiError> {
    let tenant_id = session
        .context
        .tenant_id
        .ok_or_else(|| ApiError::unauthorized("AUTH_TENANT_REQUIRED", "请先选择租户"))?;
    let view = me_account(state.db().admin(), session.context.user_id, tenant_id)
        .await
        .map_err(map_me_error)?;

    Ok(Json(MeResponse {
        user_id: view.user_id.to_string(),
        email: view.email,
        tenant_id: view.tenant_id.to_string(),
        tenant_name: view.tenant_name,
        role: view.role,
        quota: QuotaResponse {
            limit: view.quota.limit,
            used: view.quota.used,
            remaining: view.quota.remaining,
            reset_at: view.quota.reset_at.to_rfc3339(),
        },
    }))
}

fn map_me_error(error: MeError) -> ApiError {
    match error {
        MeError::NotFound => ApiError::not_found(None),
        MeError::Database { source } => {
            tracing::error!(%source, "me database error");
            ApiError::internal("/me 查询失败")
        }
        MeError::Quota { source } => {
            tracing::error!(%source, "me quota error");
            ApiError::internal("/me 查询失败")
        }
    }
}
