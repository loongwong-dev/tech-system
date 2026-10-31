//! 生成接口：认证、跨租户、SSE、幂等、重生成上限与批量任务。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use api::{AppState, build_router};
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use db::{AdminPool, Db, DbConfig};
use platform::{RegisteredAccount, RegistrationInput, generate_invite_code, register};
use serde_json::Value;
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

async fn seed_lesson(pool: &PgPool, tenant_id: Uuid, count: usize) -> (Uuid, Vec<Uuid>) {
    let student_id = Uuid::new_v4();
    sqlx::query("INSERT INTO students (id, tenant_id, name) VALUES ($1, $2, $3)")
        .bind(student_id)
        .bind(tenant_id)
        .bind("学员")
        .execute(pool)
        .await
        .expect("创建学员失败");
    let class_id = Uuid::new_v4();
    sqlx::query("INSERT INTO classes (id, tenant_id, name) VALUES ($1, $2, $3)")
        .bind(class_id)
        .bind(tenant_id)
        .bind("班级")
        .execute(pool)
        .await
        .expect("创建班级失败");
    let lesson_id = Uuid::new_v4();
    sqlx::query("INSERT INTO lessons (id, tenant_id, class_id, lesson_date) VALUES ($1, $2, $3, CURRENT_DATE)")
        .bind(lesson_id)
        .bind(tenant_id)
        .bind(class_id)
        .execute(pool)
        .await
        .expect("创建课次失败");

    let mut feedback_ids = Vec::new();
    for index in 0..count {
        let feedback_id = Uuid::new_v4();
        let student_id = if index == 0 {
            student_id
        } else {
            let id = Uuid::new_v4();
            sqlx::query("INSERT INTO students (id, tenant_id, name) VALUES ($1, $2, $3)")
                .bind(id)
                .bind(tenant_id)
                .bind(format!("学员 {index}"))
                .execute(pool)
                .await
                .expect("创建学员失败");
            id
        };
        sqlx::query(
            "INSERT INTO feedbacks (id, tenant_id, lesson_id, student_id, notes, knowledge_tags)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(feedback_id)
        .bind(tenant_id)
        .bind(lesson_id)
        .bind(student_id)
        .bind("今天讲判别式，前半节课听懂了，后面练习跟不上")
        .bind(vec!["偶有走神".to_owned(), "存在薄弱点".to_owned()])
        .execute(pool)
        .await
        .expect("创建反馈失败");
        feedback_ids.push(feedback_id);
    }
    (lesson_id, feedback_ids)
}

fn generation_request(
    path: &str,
    cookie: Option<&str>,
    idempotency: Option<&str>,
) -> Request<Body> {
    let mut builder = Request::builder().method("POST").uri(path);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("sid={cookie}"));
    }
    if let Some(key) = idempotency {
        builder = builder.header("Idempotency-Key", key);
    }
    builder.body(Body::empty()).expect("构造生成请求失败")
}

fn job_request(job_id: &str, cookie: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder()
        .method("GET")
        .uri(format!("/api/v1/jobs/{job_id}"));
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("sid={cookie}"));
    }
    builder.body(Body::empty()).expect("构造任务请求失败")
}

async fn body_text(response: Response) -> String {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("读取响应体失败");
    String::from_utf8(bytes.to_vec()).expect("响应不是 UTF-8")
}

async fn body_json(response: Response) -> Value {
    serde_json::from_str(&body_text(response).await).expect("解析 JSON 失败")
}

#[sqlx::test(migrations = false)]
async fn generation_endpoints_require_session_and_tenant(pool: PgPool) {
    let (_, app) = setup(&pool).await;
    let feedback_id = Uuid::new_v4();
    let lesson_id = Uuid::new_v4();

    let single = app
        .clone()
        .oneshot(generation_request(
            &format!("/api/v1/feedbacks/{feedback_id}/generations"),
            None,
            None,
        ))
        .await
        .expect("单份请求失败");
    assert_eq!(single.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body_json(single).await["code"], "AUTH_SESSION_EXPIRED");

    let batch = app
        .clone()
        .oneshot(generation_request(
            &format!("/api/v1/lessons/{lesson_id}/generations"),
            None,
            None,
        ))
        .await
        .expect("批量请求失败");
    assert_eq!(batch.status(), StatusCode::UNAUTHORIZED);

    let job = app
        .oneshot(job_request(&Uuid::new_v4().to_string(), None))
        .await
        .expect("任务请求失败");
    assert_eq!(job.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = false)]
async fn single_generation_streams_and_persists_without_overwriting_notes(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let (_, feedback_ids) = seed_lesson(&pool, account.tenant_id, 1).await;
    let feedback_id = feedback_ids[0];

    let response = app
        .oneshot(generation_request(
            &format!("/api/v1/feedbacks/{feedback_id}/generations"),
            Some(account.session_token.as_str()),
            None,
        ))
        .await
        .expect("生成请求失败");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .expect("content-type 非文本"),
        "text/event-stream"
    );
    let body = body_text(response).await;
    assert!(body.contains("event: delta"));
    assert!(body.contains("event: done"));
    assert!(body.contains("\"generatedBy\":\"template\""));
    assert!(body.contains("今天讲判别式"));

    let (status, generated_by, notes, rendered): (String, String, String, Value) =
        sqlx::query_as("SELECT status, generated_by, notes, rendered FROM feedbacks WHERE id = $1")
            .bind(feedback_id)
            .fetch_one(&pool)
            .await
            .expect("读取反馈失败");
    assert_eq!(status, "generated");
    assert_eq!(generated_by, "template");
    assert_eq!(notes, "今天讲判别式，前半节课听懂了，后面练习跟不上");
    assert!(
        rendered["text"]
            .as_str()
            .is_some_and(|text| text.contains("建议"))
    );
}

#[sqlx::test(migrations = false)]
async fn idempotency_key_reuses_text_without_incrementing_regeneration(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let (_, feedback_ids) = seed_lesson(&pool, account.tenant_id, 1).await;
    let feedback_id = feedback_ids[0];
    let path = format!("/api/v1/feedbacks/{feedback_id}/generations");

    let first = app
        .clone()
        .oneshot(generation_request(
            &path,
            Some(account.session_token.as_str()),
            Some("same-key"),
        ))
        .await
        .expect("首次生成失败");
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = body_text(first).await;
    assert!(first_body.contains("\"reused\":false"));

    let second = app
        .oneshot(generation_request(
            &path,
            Some(account.session_token.as_str()),
            Some("same-key"),
        ))
        .await
        .expect("重复生成失败");
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = body_text(second).await;
    assert!(second_body.contains("\"reused\":true"));

    let regen_count: i16 = sqlx::query_scalar("SELECT regen_count FROM feedbacks WHERE id = $1")
        .bind(feedback_id)
        .fetch_one(&pool)
        .await
        .expect("读取重生成次数失败");
    assert_eq!(regen_count, 0);
}

#[sqlx::test(migrations = false)]
async fn regeneration_stops_after_three_attempts(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let (_, feedback_ids) = seed_lesson(&pool, account.tenant_id, 1).await;
    let feedback_id = feedback_ids[0];
    let path = format!("/api/v1/feedbacks/{feedback_id}/generations");

    for _ in 0..4 {
        let response = app
            .clone()
            .oneshot(generation_request(
                &path,
                Some(account.session_token.as_str()),
                None,
            ))
            .await
            .expect("生成请求失败");
        assert_eq!(response.status(), StatusCode::OK);
    }

    let response = app
        .oneshot(generation_request(
            &path,
            Some(account.session_token.as_str()),
            None,
        ))
        .await
        .expect("超限请求失败");
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(response).await["code"],
        "CONFLICT_REGENERATION_LIMIT"
    );
}

#[sqlx::test(migrations = false)]
async fn batch_creates_or_reuses_job_and_progress_is_tenant_scoped(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let (lesson_id, _) = seed_lesson(&pool, account.tenant_id, 2).await;
    let path = format!("/api/v1/lessons/{lesson_id}/generations");

    let first = app
        .clone()
        .oneshot(generation_request(
            &path,
            Some(account.session_token.as_str()),
            Some("batch-key"),
        ))
        .await
        .expect("批量请求失败");
    assert_eq!(first.status(), StatusCode::ACCEPTED);
    let first_json = body_json(first).await;
    assert_eq!(first_json["total"], 2);
    assert_eq!(first_json["done"], 0);
    assert_eq!(first_json["failed"], 0);
    let job_id = first_json["jobId"].as_str().expect("缺少 jobId").to_owned();

    let second = app
        .clone()
        .oneshot(generation_request(
            &path,
            Some(account.session_token.as_str()),
            Some("batch-key"),
        ))
        .await
        .expect("重复批量请求失败");
    assert_eq!(second.status(), StatusCode::ACCEPTED);
    assert_eq!(body_json(second).await["jobId"], job_id);

    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM jobs WHERE kind = 'feedback.generate_batch'")
            .fetch_one(&pool)
            .await
            .expect("统计任务失败");
    assert_eq!(count, 1);

    let progress = app
        .clone()
        .oneshot(job_request(&job_id, Some(account.session_token.as_str())))
        .await
        .expect("查询进度失败");
    assert_eq!(progress.status(), StatusCode::OK);
    let progress_json = body_json(progress).await;
    assert_eq!(progress_json["total"], 2);
    assert_eq!(progress_json["done"], 0);

    let foreign = app
        .oneshot(job_request(&job_id, Some(other.session_token.as_str())))
        .await
        .expect("跨租户查询失败");
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);
}

#[sqlx::test(migrations = false)]
async fn cross_tenant_generation_returns_not_found(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let owner = create_account(&admin, "owner@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let (lesson_id, feedback_ids) = seed_lesson(&pool, owner.tenant_id, 1).await;

    let single = app
        .clone()
        .oneshot(generation_request(
            &format!("/api/v1/feedbacks/{}/generations", feedback_ids[0]),
            Some(other.session_token.as_str()),
            None,
        ))
        .await
        .expect("跨租户单份请求失败");
    assert_eq!(single.status(), StatusCode::NOT_FOUND);

    let batch = app
        .oneshot(generation_request(
            &format!("/api/v1/lessons/{lesson_id}/generations"),
            Some(other.session_token.as_str()),
            None,
        ))
        .await
        .expect("跨租户批量请求失败");
    assert_eq!(batch.status(), StatusCode::NOT_FOUND);
}
