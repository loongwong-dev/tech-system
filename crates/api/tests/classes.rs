//! 班级 CRUD、删除约束与租户隔离测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use api::{AppState, build_router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use db::{AdminPool, Db, DbConfig};
use platform::{RegisteredAccount, RegistrationInput, generate_invite_code, register};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

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

fn request(method: &str, path: &str, cookie: Option<&str>, body: Option<Value>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("sid={cookie}"));
    }
    match body {
        Some(body) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body.to_string()))
            .expect("构造请求失败"),
        None => builder.body(Body::empty()).expect("构造请求失败"),
    }
}

async fn body_json(response: Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("读取响应体失败");
    serde_json::from_slice(&bytes).expect("解析 JSON 失败")
}

#[sqlx::test(migrations = false)]
async fn class_crud_and_validation(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();

    let empty = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/classes",
            Some(token),
            Some(json!({"name": "   "})),
        ))
        .await
        .expect("空名称请求失败");
    assert_eq!(empty.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/classes",
            Some(token),
            Some(json!({"name": " 初二数学 ", "subject": "数学"})),
        ))
        .await
        .expect("创建班级失败");
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = body_json(created).await;
    assert_eq!(created["name"], "初二数学");
    let class_id = created["id"].as_str().expect("缺少 id").to_owned();

    let updated = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/v1/classes/{class_id}"),
            Some(token),
            Some(json!({"name": "初三数学", "subject": "数学"})),
        ))
        .await
        .expect("更新班级失败");
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(body_json(updated).await["name"], "初三数学");

    let listed = app
        .oneshot(request("GET", "/api/v1/classes", Some(token), None))
        .await
        .expect("列表失败");
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(body_json(listed).await["items"][0]["name"], "初三数学");
}

#[sqlx::test(migrations = false)]
async fn class_with_lessons_cannot_be_deleted(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();
    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/classes",
            Some(token),
            Some(json!({"name": "班级"})),
        ))
        .await
        .expect("创建班级失败");
    let class_id: Uuid = body_json(created).await["id"]
        .as_str()
        .expect("缺少 id")
        .parse()
        .expect("id 非法");

    sqlx::query(
        "INSERT INTO lessons (tenant_id, class_id, lesson_date) VALUES ($1, $2, CURRENT_DATE)",
    )
    .bind(account.tenant_id)
    .bind(class_id)
    .execute(&pool)
    .await
    .expect("创建课次失败");

    let conflict = app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/v1/classes/{class_id}"),
            Some(token),
            None,
        ))
        .await
        .expect("删除班级失败");
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(conflict).await["code"],
        "CONFLICT_CLASS_HAS_LESSONS"
    );

    let lessons: i64 = sqlx::query_scalar("SELECT count(*) FROM lessons WHERE class_id = $1")
        .bind(class_id)
        .fetch_one(&pool)
        .await
        .expect("统计课次失败");
    assert_eq!(lessons, 1, "删除失败时不应级联删除课次");

    sqlx::query("DELETE FROM lessons WHERE class_id = $1")
        .bind(class_id)
        .execute(&pool)
        .await
        .expect("清理课次失败");
    let deleted = app
        .oneshot(request(
            "DELETE",
            &format!("/api/v1/classes/{class_id}"),
            Some(token),
            None,
        ))
        .await
        .expect("删除空班级失败");
    assert_eq!(deleted.status(), StatusCode::NO_CONTENT);
}

#[sqlx::test(migrations = false)]
async fn class_cross_tenant_and_unauthenticated_access_are_rejected(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let owner = create_account(&admin, "owner@example.com").await;
    let other = create_account(&admin, "other@example.com").await;

    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/classes",
            Some(owner.session_token.as_str()),
            Some(json!({"name": "班级"})),
        ))
        .await
        .expect("创建班级失败");
    let class_id = body_json(created).await["id"]
        .as_str()
        .expect("缺少 id")
        .to_owned();

    let unauthenticated = app
        .clone()
        .oneshot(request("GET", "/api/v1/classes", None, None))
        .await
        .expect("未认证列表失败");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let foreign = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/v1/classes/{class_id}"),
            Some(other.session_token.as_str()),
            Some(json!({"name": "越权"})),
        ))
        .await
        .expect("跨租户更新失败");
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let foreign_delete = app
        .oneshot(request(
            "DELETE",
            &format!("/api/v1/classes/{class_id}"),
            Some(other.session_token.as_str()),
            None,
        ))
        .await
        .expect("跨租户删除失败");
    assert_eq!(foreign_delete.status(), StatusCode::NOT_FOUND);
}
