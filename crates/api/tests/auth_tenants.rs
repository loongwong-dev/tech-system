//! 当前用户租户列表接口测试。

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

async fn body_json(response: Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("解析 JSON 失败")
}

#[sqlx::test(migrations = false)]
async fn lists_all_active_memberships_for_current_user(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let other = create_account(&admin, "other@example.com").await;

    sqlx::query("INSERT INTO memberships (tenant_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(other.tenant_id)
        .bind(account.user_id)
        .execute(&pool)
        .await
        .expect("添加 membership 失败");
    // 冻结租户不应出现在可切换列表。
    sqlx::query("UPDATE tenants SET status = 'frozen' WHERE id = $1")
        .bind(other.tenant_id)
        .execute(&pool)
        .await
        .expect("冻结租户失败");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/auth/tenants")
                .header(
                    header::COOKIE,
                    format!("sid={}", account.session_token.as_str()),
                )
                .body(Body::empty())
                .expect("构造请求失败"),
        )
        .await
        .expect("租户列表请求失败");
    assert_eq!(response.status(), StatusCode::OK);
    let json = body_json(response).await;
    assert_eq!(json["items"].as_array().unwrap().len(), 1);
    assert_eq!(json["items"][0]["id"], account.tenant_id.to_string());
    assert_eq!(json["items"][0]["role"], "owner");

    let unauthenticated = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/auth/tenants")
                .body(Body::empty())
                .expect("构造请求失败"),
        )
        .await
        .expect("未认证请求失败");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}
