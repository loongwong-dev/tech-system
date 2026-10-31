//! axum 路由装配、中间件与 fallback。

use axum::Router;
use axum::extract::OriginalUri;
use axum::routing::{get, post, put};
use tower_http::request_id::{
    MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer,
};
use tower_http::trace::TraceLayer;

use crate::routes::auth::{login, logout, register, switch_tenant};
use crate::routes::generation::{generate_feedback, generate_lesson, job_progress};
use crate::routes::health::{healthz, readyz};
use crate::routes::me::me;
use crate::routes::students::{
    archive, create as create_student, delete as delete_student, index as index_students,
    update as update_student,
};
use crate::{ApiError, AppState};

/// 构建对外路由。
pub fn build_router(state: AppState) -> Router {
    let auth: Router<AppState> = Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/tenant", post(switch_tenant));
    let api_v1: Router<AppState> = Router::new()
        .route("/me", get(me))
        .route("/students", get(index_students).post(create_student))
        .route("/students/{id}", put(update_student).delete(delete_student))
        .route("/students/{id}/archive", post(archive))
        .route("/feedbacks/{id}/generations", post(generate_feedback))
        .route("/lessons/{id}/generations", post(generate_lesson))
        .route("/jobs/{id}", get(job_progress));
    let admin: Router<AppState> = Router::new();
    let public: Router<AppState> = Router::new();

    Router::<AppState>::new()
        .nest("/auth", auth)
        .nest("/api/v1", api_v1)
        .nest("/admin/api/v1", admin)
        .nest("/s", public)
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .fallback(not_found)
        // 最外层生成请求 ID，内层把同一个 ID 回写到响应头。
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(
            TraceLayer::new_for_http().make_span_with(|request: &axum::http::Request<_>| {
                let request_id = request
                    .extensions()
                    .get::<RequestId>()
                    .and_then(|id| id.header_value().to_str().ok())
                    .unwrap_or("-");
                tracing::info_span!(
                    "http_request",
                    request_id = %request_id,
                    method = %request.method(),
                    uri = %request.uri()
                )
            }),
        )
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .with_state(state)
}

async fn not_found(OriginalUri(uri): OriginalUri) -> ApiError {
    ApiError::not_found(Some(uri.path().to_owned()))
}
