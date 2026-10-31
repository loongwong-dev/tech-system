//! 邀请码注册流程集成测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use api::{AppState, build_router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use db::{AdminPool, Db, DbConfig};
use platform::generate_invite_code;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;

async fn setup(pool: &PgPool) -> (AdminPool, Router) {
    db::run_migrations(pool).await.expect("迁移失败");
    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    let admin = db.admin().clone();
    (admin, build_router(AppState::new(db)))
}

fn register_request(email: &str, password: &str, invite_code: &str) -> Request<Body> {
    let body = serde_json::json!({
        "email": email,
        "password": password,
        "inviteCode": invite_code,
    });
    Request::builder()
        .method("POST")
        .uri("/auth/register")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("构造请求失败")
}

async fn body_json(response: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("解析 JSON 失败")
}

async fn count(pool: &PgPool, table: &str) -> i64 {
    let query = format!("SELECT count(*) FROM {table}");
    sqlx::query_scalar(sqlx::AssertSqlSafe(query))
        .fetch_one(pool)
        .await
        .expect("统计失败")
}

#[sqlx::test(migrations = false)]
async fn registers_user_tenant_owner_and_session(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let invite = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败")
        .code;

    let response = app
        .oneshot(register_request(
            "Teacher@Example.com",
            "a-long-enough-password",
            &invite,
        ))
        .await
        .expect("请求失败");

    assert_eq!(response.status(), StatusCode::CREATED);
    let cookie = response.headers()[header::SET_COOKIE]
        .to_str()
        .expect("Cookie 非文本")
        .to_owned();
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("Secure"));
    assert!(cookie.contains("SameSite=Lax"));
    assert!(cookie.contains("Path=/"));
    assert!(cookie.contains("Max-Age=2592000"));

    let token = cookie
        .split(';')
        .next()
        .and_then(|value| value.strip_prefix("sid="))
        .expect("缺少 sid")
        .to_owned();
    let json = body_json(response).await;
    assert!(
        json["userId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
    assert!(
        json["tenantId"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );

    let email: String = sqlx::query_scalar("SELECT email FROM users")
        .fetch_one(&pool)
        .await
        .expect("读取用户失败");
    assert_eq!(email, "teacher@example.com");

    let password_hash: String = sqlx::query_scalar("SELECT password_hash FROM users")
        .fetch_one(&pool)
        .await
        .expect("读取密码哈希失败");
    assert!(password_hash.starts_with("$argon2id$"));

    let token_hash: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM auth_sessions")
        .fetch_one(&pool)
        .await
        .expect("读取会话失败");
    assert_eq!(token_hash, Sha256::digest(token.as_bytes()).to_vec());

    assert_eq!(count(&pool, "users").await, 1);
    assert_eq!(count(&pool, "tenants").await, 1);
    assert_eq!(count(&pool, "memberships").await, 1);
    assert_eq!(count(&pool, "auth_sessions").await, 1);

    let invite_hash = Sha256::digest(invite.as_bytes()).to_vec();
    let used_count: i32 =
        sqlx::query_scalar("SELECT used_count FROM invite_codes WHERE code_hash = $1")
            .bind(invite_hash)
            .fetch_one(&pool)
            .await
            .expect("读取邀请码失败");
    assert_eq!(used_count, 1);
}

#[sqlx::test(migrations = false)]
async fn invalid_invite_rolls_back_everything(pool: PgPool) {
    let (_admin, app) = setup(&pool).await;

    let response = app
        .oneshot(register_request(
            "teacher@example.com",
            "a-long-enough-password",
            "0000000000",
        ))
        .await
        .expect("请求失败");

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let json = body_json(response).await;
    assert_eq!(json["code"], "CONFLICT_INVITE_CODE_INVALID");
    assert_eq!(count(&pool, "users").await, 0);
    assert_eq!(count(&pool, "tenants").await, 0);
    assert_eq!(count(&pool, "auth_sessions").await, 0);
}

#[sqlx::test(migrations = false)]
async fn weak_password_is_rejected_before_consuming_invite(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let invite = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败")
        .code;

    let response = app
        .oneshot(register_request("teacher@example.com", "short", &invite))
        .await
        .expect("请求失败");

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(count(&pool, "users").await, 0);

    let invite_hash = Sha256::digest(invite.as_bytes()).to_vec();
    let used_count: i32 =
        sqlx::query_scalar("SELECT used_count FROM invite_codes WHERE code_hash = $1")
            .bind(invite_hash)
            .fetch_one(&pool)
            .await
            .expect("读取邀请码失败");
    assert_eq!(used_count, 0);
}

#[sqlx::test(migrations = false)]
async fn duplicate_email_rolls_back_second_registration(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let first_invite = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败")
        .code;
    let second_invite = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败")
        .code;

    let first = app
        .clone()
        .oneshot(register_request(
            "Teacher@Example.com",
            "a-long-enough-password",
            &first_invite,
        ))
        .await
        .expect("首次注册失败");
    assert_eq!(first.status(), StatusCode::CREATED);

    let second = app
        .oneshot(register_request(
            "teacher@example.com",
            "another-long-password",
            &second_invite,
        ))
        .await
        .expect("重复注册请求失败");
    assert_eq!(second.status(), StatusCode::CONFLICT);
    let json = body_json(second).await;
    assert_eq!(json["code"], "CONFLICT_EMAIL_EXISTS");
    assert_eq!(count(&pool, "users").await, 1);
    assert_eq!(count(&pool, "tenants").await, 1);

    let second_hash = Sha256::digest(second_invite.as_bytes()).to_vec();
    let used_count: i32 =
        sqlx::query_scalar("SELECT used_count FROM invite_codes WHERE code_hash = $1")
            .bind(second_hash)
            .fetch_one(&pool)
            .await
            .expect("读取邀请码失败");
    assert_eq!(used_count, 0);
}
