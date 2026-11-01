//! 反馈草稿读写：幂等 upsert、状态校验、跨租户隔离与并发。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashSet;

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

async fn create_class(app: &Router, token: &str, name: &str) -> String {
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/classes",
            Some(token),
            Some(json!({ "name": name })),
        ))
        .await
        .expect("创建班级失败");
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .expect("缺少班级 id")
        .to_owned()
}

async fn create_lesson(app: &Router, token: &str, class_id: &str) -> String {
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/lessons",
            Some(token),
            Some(json!({ "classId": class_id, "lessonDate": "2026-05-01" })),
        ))
        .await
        .expect("创建课次失败");
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .expect("缺少课次 id")
        .to_owned()
}

async fn create_student(app: &Router, token: &str, name: &str) -> String {
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/students",
            Some(token),
            Some(json!({ "name": name })),
        ))
        .await
        .expect("创建学员失败");
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .expect("缺少学员 id")
        .to_owned()
}

async fn list_feedbacks(app: &Router, token: &str, lesson_id: &str) -> Value {
    let response = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/v1/lessons/{lesson_id}/feedbacks"),
            Some(token),
            None,
        ))
        .await
        .expect("查询课次反馈失败");
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

async fn save_feedback(app: &Router, token: &str, feedback_id: &str, body: Value) -> Response {
    app.clone()
        .oneshot(request(
            "PUT",
            &format!("/api/v1/feedbacks/{feedback_id}"),
            Some(token),
            Some(body),
        ))
        .await
        .expect("保存草稿失败")
}

fn draft_body(lesson_id: &str, student_id: &str, notes: &str) -> Value {
    json!({
        "lessonId": lesson_id,
        "studentId": student_id,
        "notes": notes,
        "ratings": { "课堂表现": "专注投入" },
        "knowledgeTags": ["专注投入", "按时完成"],
        "homework": { "done": true },
    })
}

async fn item_count(app: &Router, token: &str, lesson_id: &str) -> usize {
    list_feedbacks(app, token, lesson_id).await["items"]
        .as_array()
        .expect("items 不是数组")
        .len()
}

#[sqlx::test(migrations = false)]
async fn feedback_draft_keeps_one_row_and_protects_teacher_notes(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();
    let class_id = create_class(&app, token, "初二数学").await;
    let lesson_id = create_lesson(&app, token, &class_id).await;
    let first = create_student(&app, token, "小明").await;
    let second = create_student(&app, token, "小红").await;

    assert_eq!(
        list_feedbacks(&app, token, &lesson_id).await["items"],
        json!([]),
        "课次没有反馈时返回空数组"
    );

    let draft_id = Uuid::new_v4().to_string();
    let response = save_feedback(
        &app,
        token,
        &draft_id,
        draft_body(&lesson_id, &first, "判别式前半节课听懂"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    assert_eq!(created["id"], draft_id);
    assert_eq!(created["status"], "draft");
    assert_eq!(created["notes"], "判别式前半节课听懂");
    assert_eq!(created["knowledgeTags"], json!(["专注投入", "按时完成"]));
    assert_eq!(created["ratings"], json!({ "课堂表现": "专注投入" }));
    assert_eq!(created["homework"], json!({ "done": true }));

    // 同一 id 重试（防抖重复提交）只更新，不新增行。
    let response = save_feedback(
        &app,
        token,
        &draft_id,
        draft_body(&lesson_id, &first, "判别式前半节课听懂，练习跟不上"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["id"], draft_id);
    assert_eq!(item_count(&app, token, &lesson_id).await, 1);

    // 换一个 id 写同一名学员：命中 (lesson_id, student_id) 唯一约束，合并到已有行。
    let merged_id = Uuid::new_v4().to_string();
    let response = save_feedback(
        &app,
        token,
        &merged_id,
        draft_body(&lesson_id, &first, "离线补录的要点"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let merged = body_json(response).await;
    assert_eq!(merged["id"], draft_id, "重复写入必须合并到已有行");
    assert_eq!(merged["notes"], "离线补录的要点");
    assert_eq!(item_count(&app, token, &lesson_id).await, 1);

    // 另一名学员是独立的一行。
    let response = save_feedback(
        &app,
        token,
        &Uuid::new_v4().to_string(),
        draft_body(&lesson_id, &second, "笔记规范"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(item_count(&app, token, &lesson_id).await, 2);

    // 生成只写 rendered，教师原文必须保持不变，并把状态推进到 generated。
    let generated = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/v1/feedbacks/{draft_id}/generations"),
            Some(token),
            None,
        ))
        .await
        .expect("生成反馈失败");
    assert_eq!(generated.status(), StatusCode::OK);
    let listed = list_feedbacks(&app, token, &lesson_id).await;
    let items = listed["items"].as_array().expect("items 不是数组");
    assert_eq!(items.len(), 2);
    let first_draft = items
        .iter()
        .find(|item| item["id"] == draft_id.as_str())
        .expect("缺少第一份反馈");
    assert_eq!(
        first_draft["notes"], "离线补录的要点",
        "生成不得覆盖教师原文"
    );
    assert_eq!(first_draft["status"], "generated");

    // 非 draft 状态拒绝继续编辑，同一组合的新 id 也一样。
    for id in [draft_id.as_str(), &Uuid::new_v4().to_string()] {
        let response = save_feedback(
            &app,
            token,
            id,
            draft_body(&lesson_id, &first, "生成后再改"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            body_json(response).await["code"],
            "CONFLICT_INVALID_TRANSITION"
        );
    }
    assert_eq!(item_count(&app, token, &lesson_id).await, 2);
}

#[sqlx::test(migrations = false)]
async fn feedback_draft_validates_input_and_isolates_tenants(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let owner = create_account(&admin, "owner@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let token = owner.session_token.as_str();
    let class_id = create_class(&app, token, "班级").await;
    let lesson_id = create_lesson(&app, token, &class_id).await;
    let student_id = create_student(&app, token, "小明").await;

    let foreign_lesson = create_lesson(
        &app,
        other.session_token.as_str(),
        &create_class(&app, other.session_token.as_str(), "他人班级").await,
    )
    .await;
    let foreign_student = create_student(&app, other.session_token.as_str(), "小红").await;

    let foreign = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/v1/lessons/{foreign_lesson}/feedbacks"),
            Some(token),
            None,
        ))
        .await
        .expect("跨租户课次请求失败");
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    for body in [
        draft_body(&foreign_lesson, &student_id, "跨租户课次"),
        draft_body(&lesson_id, &foreign_student, "跨租户学员"),
    ] {
        let response = save_feedback(&app, token, &Uuid::new_v4().to_string(), body).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let reused_id = Uuid::new_v4().to_string();
    let response = save_feedback(
        &app,
        token,
        &reused_id,
        draft_body(&lesson_id, &student_id, "先占位"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let second_student = create_student(&app, token, "小刚").await;
    let response = save_feedback(
        &app,
        token,
        &reused_id,
        draft_body(&lesson_id, &second_student, "id 复用"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body_json(response).await["code"],
        "CONFLICT_FEEDBACK_ID_REUSED"
    );

    let mut oversized_notes = draft_body(&lesson_id, &second_student, "占位");
    oversized_notes["notes"] = json!("字".repeat(2001));
    let mut too_many_tags = draft_body(&lesson_id, &second_student, "占位");
    too_many_tags["knowledgeTags"] = json!(vec!["专注投入"; 51]);
    let mut long_tag = draft_body(&lesson_id, &second_student, "占位");
    long_tag["knowledgeTags"] = json!(["标".repeat(81)]);
    let mut array_ratings = draft_body(&lesson_id, &second_student, "占位");
    array_ratings["ratings"] = json!(["专注投入"]);
    let mut string_homework = draft_body(&lesson_id, &second_student, "占位");
    string_homework["homework"] = json!("已提交");

    for (body, field) in [
        (oversized_notes, "notes"),
        (too_many_tags, "knowledgeTags"),
        (long_tag, "knowledgeTags"),
        (array_ratings, "ratings"),
        (string_homework, "homework"),
    ] {
        let response = save_feedback(&app, token, &Uuid::new_v4().to_string(), body).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "字段 {field} 应返回 422"
        );
        assert_eq!(body_json(response).await["errors"][0]["field"], field);
    }

    // 缺少必填字段与非法 id 由反序列化或 UUID 解析拦下。
    let missing_field = json!({
        "lessonId": lesson_id,
        "notes": "缺 studentId",
        "ratings": {},
        "knowledgeTags": [],
        "homework": {},
    });
    let response = save_feedback(&app, token, &Uuid::new_v4().to_string(), missing_field).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let response = save_feedback(
        &app,
        token,
        "not-a-uuid",
        draft_body(&lesson_id, &second_student, "占位"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(body_json(response).await["errors"][0]["field"], "id");

    let unauthenticated = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/v1/lessons/{lesson_id}/feedbacks"),
            None,
            None,
        ))
        .await
        .expect("未认证请求失败");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let unauthenticated = save_feedback(
        &app,
        "",
        &Uuid::new_v4().to_string(),
        draft_body(&lesson_id, &second_student, "未认证"),
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    // 无效课次 id 走校验分支而不是 500。
    let invalid_lesson = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/v1/lessons/not-a-uuid/feedbacks",
            Some(token),
            None,
        ))
        .await
        .expect("非法课次请求失败");
    assert_eq!(invalid_lesson.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = false)]
async fn concurrent_feedback_draft_writes_keep_single_row(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str().to_owned();
    let class_id = create_class(&app, token.as_str(), "并发班级").await;
    let lesson_id = create_lesson(&app, token.as_str(), &class_id).await;
    let student_id = create_student(&app, token.as_str(), "小明").await;

    // 不同 id、同一学员：唯一索引串行化，最终只保留一行，全部返回同一个 id。
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..8 {
        let app = app.clone();
        let token = token.clone();
        let lesson_id = lesson_id.clone();
        let student_id = student_id.clone();
        tasks.spawn(async move {
            let response = save_feedback(
                &app,
                token.as_str(),
                &Uuid::new_v4().to_string(),
                draft_body(&lesson_id, &student_id, &format!("并发 {index}")),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
            body_json(response).await["id"]
                .as_str()
                .expect("缺少反馈 id")
                .to_owned()
        });
    }
    let mut ids = HashSet::new();
    while let Some(result) = tasks.join_next().await {
        ids.insert(result.expect("并发写入任务失败"));
    }
    assert_eq!(ids.len(), 1, "并发 upsert 必须落到同一行");

    // 同一 id 重复提交同样只产生一行。
    let shared_id = ids.iter().next().expect("缺少反馈 id").clone();
    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..5 {
        let app = app.clone();
        let token = token.clone();
        let lesson_id = lesson_id.clone();
        let student_id = student_id.clone();
        let shared_id = shared_id.clone();
        tasks.spawn(async move {
            let response = save_feedback(
                &app,
                token.as_str(),
                &shared_id,
                draft_body(&lesson_id, &student_id, &format!("重试 {index}")),
            )
            .await;
            assert_eq!(response.status(), StatusCode::OK);
        });
    }
    while let Some(result) = tasks.join_next().await {
        result.expect("并发重试任务失败");
    }

    let listed = list_feedbacks(&app, token.as_str(), &lesson_id).await;
    let items = listed["items"].as_array().expect("items 不是数组");
    assert_eq!(items.len(), 1, "并发写入后不允许出现重复行");
    assert_eq!(items[0]["id"], shared_id);
    let notes = items[0]["notes"].as_str().expect("缺少教师原文");
    assert!(
        notes.starts_with("并发 ") || notes.starts_with("重试 "),
        "并发生成的最后写入应来自本次请求，实际为 {notes}"
    );
}
