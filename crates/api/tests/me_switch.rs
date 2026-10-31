//! 租户上下文、/me 与租户切换集成测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use api::{AppState, build_router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use db::{AdminPool, Db, DbConfig};
use platform::{RegisteredAccount, RegistrationInput, generate_invite_code, register};
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup(pool: &PgPool) -> (AdminPool, Router) {
    db::run_migrations(pool).await.expect("迁移失败");
    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    let admin = db.admin().clone();
    (admin, build_router(AppState::new(db)))
}

async fn create_account(admin: &AdminPool, email: &str) -> RegisteredAccount {
    let invite = generate_invite_code(admin, None, 1, None)
        .await
        .expect("生成邀请码失败")
        .code;
    register(
        admin,
        RegistrationInput {
            email: email.to_owned(),
            password: "a-long-enough-password".to_owned(),
            invite_code: invite,
        },
    )
    .await
    .expect("注册失败")
}

fn me_request(cookie: Option<&str>, query: Option<&str>) -> Request<Body> {
    let uri = match query {
        Some(query) => format!("/api/v1/me?{query}"),
        None => "/api/v1/me".to_owned(),
    };
    let mut builder = Request::builder().method("GET").uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("sid={cookie}"));
    }
    builder.body(Body::empty()).expect("构造 /me 请求失败")
}

fn switch_request(cookie: &str, tenant_id: &str) -> Request<Body> {
    let body = serde_json::json!({ "tenantId": tenant_id });
    Request::builder()
        .method("POST")
        .uri("/auth/tenant")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("sid={cookie}"))
        .body(Body::from(body.to_string()))
        .expect("构造切换请求失败")
}

fn session_cookie(response: &Response) -> String {
    response.headers()[header::SET_COOKIE]
        .to_str()
        .expect("Cookie 非文本")
        .to_owned()
}

fn token_from_cookie(cookie: &str) -> String {
    cookie
        .split(';')
        .next()
        .and_then(|value| value.strip_prefix("sid="))
        .expect("缺少 sid")
        .to_owned()
}

async fn body_json(response: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("解析 JSON 失败")
}

#[sqlx::test(migrations = false)]
async fn me_returns_session_tenant_role_and_quota(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let token = account.session_token.as_str();

    let response = app
        .clone()
        .oneshot(me_request(
            Some(token),
            Some(&format!("tenantId={}", other.tenant_id)),
        ))
        .await
        .expect("/me 请求失败");
    assert_eq!(response.status(), StatusCode::OK);

    let json = body_json(response).await;
    let expected_tenant = account.tenant_id.to_string();
    assert_eq!(json["email"], "teacher@example.com");
    assert_eq!(json["tenantId"].as_str(), Some(expected_tenant.as_str()));
    assert_eq!(json["tenantName"], "我的租户");
    assert_eq!(json["role"], "owner");
    assert_eq!(json["quota"]["limit"], 15);
    assert_eq!(json["quota"]["used"], 0);
    assert_eq!(json["quota"]["remaining"], 15);
    assert!(
        json["quota"]["resetAt"]
            .as_str()
            .is_some_and(|v| !v.is_empty())
    );

    let missing = app
        .oneshot(me_request(None, None))
        .await
        .expect("/me 无 Cookie 请求失败");
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(missing).await["code"], "AUTH_SESSION_EXPIRED");
}

#[sqlx::test(migrations = false)]
async fn non_member_switch_is_forbidden_and_old_session_stays_valid(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let token = account.session_token.as_str();

    let response = app
        .clone()
        .oneshot(switch_request(token, &other.tenant_id.to_string()))
        .await
        .expect("切换请求失败");
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(body_json(response).await["code"], "FORBIDDEN_TENANT");

    let me = app
        .oneshot(me_request(Some(token), None))
        .await
        .expect("/me 请求失败");
    assert_eq!(me.status(), StatusCode::OK);
    let expected_tenant = account.tenant_id.to_string();
    assert_eq!(
        body_json(me).await["tenantId"].as_str(),
        Some(expected_tenant.as_str())
    );
}

#[sqlx::test(migrations = false)]
async fn switch_rotates_session_and_updates_tenant(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    sqlx::query("INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(other.tenant_id)
        .bind(account.user_id)
        .execute(&pool)
        .await
        .expect("添加 membership 失败");

    let old_token = account.session_token.as_str();
    let response = app
        .clone()
        .oneshot(switch_request(old_token, &other.tenant_id.to_string()))
        .await
        .expect("切换请求失败");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let new_token = token_from_cookie(&session_cookie(&response));
    assert_ne!(new_token, old_token);

    let old_me = app
        .clone()
        .oneshot(me_request(Some(old_token), None))
        .await
        .expect("旧 token /me 请求失败");
    assert_eq!(old_me.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(old_me).await["code"], "AUTH_SESSION_EXPIRED");

    let new_me = app
        .oneshot(me_request(Some(&new_token), None))
        .await
        .expect("新 token /me 请求失败");
    assert_eq!(new_me.status(), StatusCode::OK);
    let json = body_json(new_me).await;
    let expected_tenant = other.tenant_id.to_string();
    assert_eq!(json["tenantId"].as_str(), Some(expected_tenant.as_str()));
    assert_eq!(json["role"], "member");
}
