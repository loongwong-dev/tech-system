//! 健康检查路由。

use axum::extract::State;
use axum::http::StatusCode;

use crate::AppState;

#[utoipa::path(
    get,
    path = "/healthz",
    operation_id = "healthz",
    tag = "health",
    responses((status = 200, description = "进程存活"))
)]
/// 存活探针：只检查进程是否还能响应。
pub(crate) async fn healthz() -> StatusCode {
    StatusCode::OK
}

#[utoipa::path(
    get,
    path = "/readyz",
    operation_id = "readyz",
    tag = "health",
    responses(
        (status = 200, description = "依赖就绪"),
        (status = 503, description = "数据库或队列表未就绪")
    )
)]
/// 就绪探针：检查双连接池与任务队列表。
pub(crate) async fn readyz(State(state): State<AppState>) -> StatusCode {
    if let Err(error) = state.db().health_check().await {
        tracing::warn!(%error, "database readiness check failed");
        return StatusCode::SERVICE_UNAVAILABLE;
    }

    // jobs 表由任务队列任务创建；迁移尚未包含该表时，就绪探针保持 503。
    match state.db().admin().table_exists("public.jobs").await {
        Ok(true) => StatusCode::OK,
        Ok(false) => {
            tracing::warn!("jobs table is missing");
            StatusCode::SERVICE_UNAVAILABLE
        }
        Err(error) => {
            tracing::warn!(%error, "queue readiness check failed");
            StatusCode::SERVICE_UNAVAILABLE
        }
    }
}
