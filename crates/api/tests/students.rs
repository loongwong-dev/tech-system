//! 学员 CRUD、归档、删除确认、级联与审计测试。

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
async fn student_crud_and_archive(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();

    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/students",
            Some(token),
            Some(json!({"name": " 小明 ", "grade": "初二", "subject": "数学"})),
        ))
        .await
        .expect("创建学员失败");
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = body_json(created).await;
    assert_eq!(created["name"], "小明");
    assert_eq!(created["grade"], "初二");
    let student_id = created["id"].as_str().expect("缺少 id").to_owned();

    let updated = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/v1/students/{student_id}"),
            Some(token),
            Some(json!({"name": "小明", "grade": "初三", "subject": "数学"})),
        ))
        .await
        .expect("更新学员失败");
    assert_eq!(updated.status(), StatusCode::OK);
    assert_eq!(body_json(updated).await["grade"], "初三");

    let listed = app
        .clone()
        .oneshot(request("GET", "/api/v1/students", Some(token), None))
        .await
        .expect("列表失败");
    assert_eq!(listed.status(), StatusCode::OK);
    assert_eq!(body_json(listed).await["items"][0]["name"], "小明");

    let archived = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/v1/students/{student_id}/archive"),
            Some(token),
            None,
        ))
        .await
        .expect("归档失败");
    assert_eq!(archived.status(), StatusCode::NO_CONTENT);

    let listed = app
        .oneshot(request("GET", "/api/v1/students", Some(token), None))
        .await
        .expect("归档后列表失败");
    assert_eq!(
        body_json(listed).await["items"].as_array().unwrap().len(),
        0
    );
}

#[sqlx::test(migrations = false)]
async fn delete_requires_exact_name_and_cascades_with_audit(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();
    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/students",
            Some(token),
            Some(json!({"name": "小明"})),
        ))
        .await
        .expect("创建学员失败");
    let student_id: Uuid = body_json(created).await["id"]
        .as_str()
        .expect("缺少 id")
        .parse()
        .expect("id 非法");

    let class_id = Uuid::new_v4();
    sqlx::query("INSERT INTO classes (id, tenant_id, name) VALUES ($1, $2, '班级')")
        .bind(class_id)
        .bind(account.tenant_id)
        .execute(&pool)
        .await
        .expect("创建班级失败");
    let lesson_id = Uuid::new_v4();
    sqlx::query("INSERT INTO lessons (id, tenant_id, class_id, lesson_date) VALUES ($1, $2, $3, CURRENT_DATE)")
        .bind(lesson_id)
        .bind(account.tenant_id)
        .bind(class_id)
        .execute(&pool)
        .await
        .expect("创建课次失败");
    let feedback_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO feedbacks (id, tenant_id, lesson_id, student_id) VALUES ($1, $2, $3, $4)",
    )
    .bind(feedback_id)
    .bind(account.tenant_id)
    .bind(lesson_id)
    .bind(student_id)
    .execute(&pool)
    .await
    .expect("创建反馈失败");
    sqlx::query("INSERT INTO share_artifacts (tenant_id, feedback_id, kind, token_hash) VALUES ($1, $2, 'link', $3)")
        .bind(account.tenant_id)
        .bind(feedback_id)
        .bind(b"student-delete-token".as_slice())
        .execute(&pool)
        .await
        .expect("创建分享物失败");

    let mismatch = app
        .clone()
        .oneshot(request(
            "DELETE",
            &format!("/api/v1/students/{student_id}"),
            Some(token),
            Some(json!({"name": "小红"})),
        ))
        .await
        .expect("删除请求失败");
    assert_eq!(mismatch.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        body_json(mismatch).await["code"],
        "VALIDATION_NAME_MISMATCH"
    );

    let deleted = app
        .oneshot(request(
            "DELETE",
            &format!("/api/v1/students/{student_id}"),
            Some(token),
            Some(json!({"name": "小明"})),
        ))
        .await
        .expect("删除请求失败");
    assert_eq!(deleted.status(), StatusCode::OK);
    assert_eq!(body_json(deleted).await["feedbackCount"], 1);

    let feedbacks: i64 = sqlx::query_scalar("SELECT count(*) FROM feedbacks WHERE student_id = $1")
        .bind(student_id)
        .fetch_one(&pool)
        .await
        .expect("统计反馈失败");
    let artifacts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM share_artifacts WHERE feedback_id = $1")
            .bind(feedback_id)
            .fetch_one(&pool)
            .await
            .expect("统计分享物失败");
    assert_eq!(feedbacks, 0);
    assert_eq!(artifacts, 0);

    let (action, before): (String, Value) = sqlx::query_as(
        "SELECT action, before FROM audit_logs WHERE tenant_id = $1 ORDER BY id DESC LIMIT 1",
    )
    .bind(account.tenant_id)
    .fetch_one(&pool)
    .await
    .expect("读取审计失败");
    assert_eq!(action, "student.deleted");
    assert_eq!(before["feedbackCount"], 1);
}

#[sqlx::test(migrations = false)]
async fn cross_tenant_and_unauthenticated_access_are_rejected(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let owner = create_account(&admin, "owner@example.com").await;
    let other = create_account(&admin, "other@example.com").await;

    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/students",
            Some(owner.session_token.as_str()),
            Some(json!({"name": "小明"})),
        ))
        .await
        .expect("创建学员失败");
    let student_id = body_json(created).await["id"]
        .as_str()
        .expect("缺少 id")
        .to_owned();

    let unauthenticated = app
        .clone()
        .oneshot(request("GET", "/api/v1/students", None, None))
        .await
        .expect("未认证列表失败");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let foreign = app
        .clone()
        .oneshot(request(
            "PUT",
            &format!("/api/v1/students/{student_id}"),
            Some(other.session_token.as_str()),
            Some(json!({"name": "越权"})),
        ))
        .await
        .expect("跨租户更新失败");
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let foreign_delete = app
        .oneshot(request(
            "DELETE",
            &format!("/api/v1/students/{student_id}"),
            Some(other.session_token.as_str()),
            Some(json!({"name": "小明"})),
        ))
        .await
        .expect("跨租户删除失败");
    assert_eq!(foreign_delete.status(), StatusCode::NOT_FOUND);
}
