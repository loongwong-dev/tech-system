//! 标签目录、标签校验与学员标签继承。

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

async fn create_lesson(app: &Router, token: &str, class_id: &str, date: &str) -> String {
    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/api/v1/lessons",
            Some(token),
            Some(json!({ "classId": class_id, "lessonDate": date })),
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

fn draft_body(lesson_id: &str, student_id: &str, tags: &[&str]) -> Value {
    json!({
        "lessonId": lesson_id,
        "studentId": student_id,
        "notes": "今天讲判别式",
        "ratings": {},
        "knowledgeTags": tags,
        "homework": {},
    })
}

async fn save_draft(app: &Router, token: &str, body: Value) -> Response {
    app.clone()
        .oneshot(request(
            "PUT",
            &format!("/api/v1/feedbacks/{}", Uuid::new_v4()),
            Some(token),
            Some(body),
        ))
        .await
        .expect("保存草稿失败")
}

async fn student_tags(app: &Router, token: &str, student_id: &str) -> Response {
    app.clone()
        .oneshot(request(
            "GET",
            &format!("/api/v1/students/{student_id}/feedback-tags"),
            Some(token),
            None,
        ))
        .await
        .expect("查询学员标签失败")
}

#[sqlx::test(migrations = false)]
async fn catalog_lists_builtin_tags_with_tones(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;

    let response = app
        .clone()
        .oneshot(request(
            "GET",
            "/api/v1/feedback-tags",
            Some(account.session_token.as_str()),
            None,
        ))
        .await
        .expect("目录请求失败");
    assert_eq!(response.status(), StatusCode::OK);
    let catalog = body_json(response).await;
    let dimensions = catalog["dimensions"]
        .as_array()
        .expect("dimensions 不是数组");
    assert_eq!(dimensions.len(), 5);
    let names: Vec<&str> = dimensions
        .iter()
        .map(|dimension| dimension["name"].as_str().expect("缺少维度名"))
        .collect();
    assert_eq!(names, ["课堂表现", "作业", "知识点", "进步", "习惯"]);

    let mut tones: Vec<(&str, &Value)> = Vec::new();
    for dimension in dimensions {
        for tag in dimension["tags"].as_array().expect("tags 不是数组") {
            tones.push((tag["name"].as_str().expect("缺少标签名"), &tag["tone"]));
        }
    }
    assert_eq!(tones.len(), 17, "目录必须可枚举全部内置标签");
    let tone_of = |name: &str| {
        tones
            .iter()
            .find(|(tag, _)| *tag == name)
            .unwrap_or_else(|| panic!("目录缺少标签 {name}"))
            .1
    };
    assert_eq!(tone_of("专注投入"), &json!("positive"));
    assert_eq!(tone_of("基本掌握"), &json!("neutral"));
    assert_eq!(tone_of("稳步提升"), &json!("neutral"));
    assert_eq!(tone_of("存在薄弱点"), &json!("negative"));

    let unauthenticated = app
        .clone()
        .oneshot(request("GET", "/api/v1/feedback-tags", None, None))
        .await
        .expect("未认证请求失败");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = false)]
async fn draft_rejects_unknown_tag_and_same_dimension_conflict(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();
    let class_id = create_class(&app, token, "初二数学").await;
    let lesson_id = create_lesson(&app, token, &class_id, "2026-06-01").await;
    let student_id = create_student(&app, token, "小明").await;

    let unknown = save_draft(
        &app,
        token,
        draft_body(&lesson_id, &student_id, &["专注投入", "无中生有"]),
    )
    .await;
    assert_eq!(unknown.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let problem = body_json(unknown).await;
    assert_eq!(problem["code"], "VALIDATION_UNKNOWN_TAG");
    assert_eq!(problem["errors"][0]["field"], "knowledgeTags");

    // 同一维度两个标签会让语气与话术矛盾，必须拒绝。
    let conflict = save_draft(
        &app,
        token,
        draft_body(&lesson_id, &student_id, &["专注投入", "偶有走神"]),
    )
    .await;
    assert_eq!(conflict.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let conflict_problem = body_json(conflict).await;
    assert_eq!(conflict_problem["code"], "VALIDATION_TAG_CONFLICT");
    assert_eq!(conflict_problem["errors"][0]["field"], "knowledgeTags");

    let accepted = save_draft(
        &app,
        token,
        draft_body(&lesson_id, &student_id, &["专注投入", "按时完成"]),
    )
    .await;
    assert_eq!(accepted.status(), StatusCode::OK);
}

#[sqlx::test(migrations = false)]
async fn inherited_tags_follow_the_latest_lesson(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let token = account.session_token.as_str();
    let class_id = create_class(&app, token, "初二数学").await;
    let student_id = create_student(&app, token, "小明").await;

    let empty = student_tags(&app, token, &student_id).await;
    assert_eq!(empty.status(), StatusCode::OK);
    let empty = body_json(empty).await;
    assert_eq!(empty["tags"], json!([]));
    assert_eq!(empty["tone"], json!("neutral"));

    // 先写新课次的反馈，再写旧课次的反馈：继承必须按课次日期取最近一次，而不是写入时间。
    let newer_lesson = create_lesson(&app, token, &class_id, "2026-06-10").await;
    let older_lesson = create_lesson(&app, token, &class_id, "2026-06-03").await;
    let newer = save_draft(
        &app,
        token,
        draft_body(&newer_lesson, &student_id, &["偶有走神", "存在薄弱点"]),
    )
    .await;
    assert_eq!(newer.status(), StatusCode::OK);
    let older = save_draft(
        &app,
        token,
        draft_body(&older_lesson, &student_id, &["专注投入", "按时完成"]),
    )
    .await;
    assert_eq!(older.status(), StatusCode::OK);

    let inherited = student_tags(&app, token, &student_id).await;
    assert_eq!(inherited.status(), StatusCode::OK);
    let inherited = body_json(inherited).await;
    assert_eq!(inherited["tags"], json!(["偶有走神", "存在薄弱点"]));
    assert_eq!(inherited["tone"], json!("negative"));

    // 重新写入新课次的草稿（合并会刷新 updated_at）：继承必须跟着最新内容走，
    // 同时排除「按写入时间取最早一行」的实现。
    let updated = save_draft(
        &app,
        token,
        draft_body(&newer_lesson, &student_id, &["需督促", "未提交"]),
    )
    .await;
    assert_eq!(updated.status(), StatusCode::OK);
    let inherited = body_json(student_tags(&app, token, &student_id).await).await;
    assert_eq!(inherited["tags"], json!(["需督促", "未提交"]));

    // 另一名学员的标签互不影响，防止查询漏掉 student_id 过滤。
    let other_student = create_student(&app, token, "小红").await;
    let other_lesson = create_lesson(&app, token, &class_id, "2026-06-17").await;
    let saved = save_draft(
        &app,
        token,
        draft_body(&other_lesson, &other_student, &["明显进步"]),
    )
    .await;
    assert_eq!(saved.status(), StatusCode::OK);
    let mine = body_json(student_tags(&app, token, &student_id).await).await;
    assert_eq!(mine["tags"], json!(["需督促", "未提交"]));
    let theirs = body_json(student_tags(&app, token, &other_student).await).await;
    assert_eq!(theirs["tags"], json!(["明显进步"]));
    assert_eq!(theirs["tone"], json!("positive"));

    // 跨租户学员按不存在处理，防止通过继承接口探测他人数据。
    let foreign = student_tags(&app, other.session_token.as_str(), &student_id).await;
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let invalid = student_tags(&app, token, "not-a-uuid").await;
    assert_eq!(invalid.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
