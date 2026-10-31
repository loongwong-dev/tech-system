//! API 骨架集成测试：路由分组、Problem Details、探针与请求 ID。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use api::{ApiError, AppState, FieldError, build_router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::{IntoResponse, Response};
use db::{Db, DbConfig};
use sqlx::PgPool;
use tower::ServiceExt;

fn test_app(pool: &PgPool) -> Router {
    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    build_router(AppState::new(db))
}

fn request(uri: &str) -> Request<Body> {
    Request::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("构造请求失败")
}

async fn body_json(response: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("解析 JSON 失败")
}

#[sqlx::test(migrations = false)]
async fn unknown_routes_return_problem_details(pool: PgPool) {
    let app = test_app(&pool);

    for uri in [
        "/auth/unknown",
        "/api/v1/unknown",
        "/admin/api/v1/unknown",
        "/s/token",
        "/unknown",
    ] {
        let response = app.clone().oneshot(request(uri)).await.expect("请求失败");
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/problem+json",
            "{uri}"
        );

        let json = body_json(response).await;
        assert_eq!(json["code"], "NOT_FOUND", "{uri}");
        assert_eq!(json["status"], 404, "{uri}");
        assert_eq!(json["instance"], uri, "{uri}");
    }
}

#[sqlx::test(migrations = false)]
async fn healthz_returns_request_id(pool: PgPool) {
    let response = test_app(&pool)
        .oneshot(request("/healthz"))
        .await
        .expect("请求失败");

    assert_eq!(response.status(), StatusCode::OK);
    let request_id = response
        .headers()
        .get("x-request-id")
        .expect("缺少 x-request-id");
    assert!(!request_id.is_empty());
}

#[sqlx::test(migrations = false)]
async fn client_request_id_is_propagated(pool: PgPool) {
    let response = test_app(&pool)
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("x-request-id", "client-request-id")
                .body(Body::empty())
                .expect("构造请求失败"),
        )
        .await
        .expect("请求失败");

    assert_eq!(response.headers()["x-request-id"], "client-request-id");
}

#[sqlx::test(migrations = false)]
async fn readyz_requires_jobs_table(pool: PgPool) {
    sqlx::query("CREATE TABLE jobs (id int)")
        .execute(&pool)
        .await
        .expect("创建 jobs 表失败");
    let ready = test_app(&pool)
        .oneshot(request("/readyz"))
        .await
        .expect("请求失败");
    assert_eq!(ready.status(), StatusCode::OK);

    sqlx::query("DROP TABLE jobs")
        .execute(&pool)
        .await
        .expect("删除 jobs 表失败");
    let not_ready = test_app(&pool)
        .oneshot(request("/readyz"))
        .await
        .expect("请求失败");
    assert_eq!(not_ready.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn validation_error_uses_problem_details() {
    let response = ApiError::validation(vec![FieldError {
        field: "email".to_owned(),
        reason: "格式无效".to_owned(),
    }])
    .into_response();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/problem+json"
    );

    let json = body_json(response).await;
    assert_eq!(json["code"], "VALIDATION_INVALID");
    assert_eq!(json["status"], 422);
    assert_eq!(json["errors"][0]["field"], "email");
}

#[tokio::test]
async fn internal_error_uses_problem_details() {
    let response = ApiError::internal("内部错误定位信息").into_response();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/problem+json"
    );

    let json = body_json(response).await;
    assert_eq!(json["code"], "INTERNAL_ERROR");
    assert_eq!(json["status"], 500);
}
