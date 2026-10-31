//! 登录、登出与会话轮换集成测试。

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

fn login_request(email: &str, password: &str, cookie: Option<&str>) -> Request<Body> {
    let body = serde_json::json!({ "email": email, "password": password });
    let mut builder = Request::builder()
        .method("POST")
        .uri("/auth/login")
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("sid={cookie}"));
    }
    builder
        .body(Body::from(body.to_string()))
        .expect("构造登录请求失败")
}

fn logout_request(cookie: &str) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri("/auth/logout")
        .header(header::COOKIE, format!("sid={cookie}"))
        .body(Body::empty())
        .expect("构造登出请求失败")
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
async fn login_rotates_old_session_and_issues_new_cookie(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let old_token = account.session_token.as_str().to_owned();

    let response = app
        .clone()
        .oneshot(login_request(
            "Teacher@Example.com",
            "a-long-enough-password",
            Some(&old_token),
        ))
        .await
        .expect("登录请求失败");
    assert_eq!(response.status(), StatusCode::OK);

    let cookie = session_cookie(&response);
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Path=/"));
    assert!(cookie.contains("Max-Age=2592000"));
    let new_token = token_from_cookie(&cookie);
    assert_ne!(new_token, old_token);

    let json = body_json(response).await;
    let user_id = account.user_id.to_string();
    let tenant_id = account.tenant_id.to_string();
    assert_eq!(json["userId"].as_str(), Some(user_id.as_str()));
    assert_eq!(json["tenantId"].as_str(), Some(tenant_id.as_str()));

    let old_logout = app
        .clone()
        .oneshot(logout_request(&old_token))
        .await
        .expect("旧会话登出请求失败");
    assert_eq!(old_logout.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(old_logout).await["code"], "AUTH_SESSION_EXPIRED");

    let new_logout = app
        .oneshot(logout_request(&new_token))
        .await
        .expect("新会话登出请求失败");
    assert_eq!(new_logout.status(), StatusCode::NO_CONTENT);
    assert!(session_cookie(&new_logout).contains("Max-Age=0"));
}

#[sqlx::test(migrations = false)]
async fn logout_revokes_and_second_attempt_is_unauthorized(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str().to_owned();

    let first = app
        .clone()
        .oneshot(logout_request(&token))
        .await
        .expect("登出请求失败");
    assert_eq!(first.status(), StatusCode::NO_CONTENT);

    let second = app
        .oneshot(logout_request(&token))
        .await
        .expect("重复登出请求失败");
    assert_eq!(second.status(), StatusCode::UNAUTHORIZED);
    let json = body_json(second).await;
    assert_eq!(json["code"], "AUTH_SESSION_EXPIRED");
}

#[sqlx::test(migrations = false)]
async fn missing_user_and_wrong_password_share_the_same_response(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let _account = create_account(&admin, "teacher@example.com").await;

    let missing = app
        .clone()
        .oneshot(login_request(
            "missing@example.com",
            "a-long-enough-password",
            None,
        ))
        .await
        .expect("不存在用户登录请求失败");
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    let missing_json = body_json(missing).await;

    let wrong = app
        .oneshot(login_request(
            "teacher@example.com",
            "wrong-long-password",
            None,
        ))
        .await
        .expect("错误密码登录请求失败");
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    let wrong_json = body_json(wrong).await;

    assert_eq!(missing_json["code"], "AUTH_INVALID_CREDENTIALS");
    assert_eq!(wrong_json["code"], "AUTH_INVALID_CREDENTIALS");
    assert_eq!(missing_json["detail"], wrong_json["detail"]);
}
