//! 课次创建、游标分页、并发与租户隔离测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashSet;
use std::time::Duration;

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
            Some(json!({"name": name})),
        ))
        .await
        .expect("创建班级失败");
    assert_eq!(response.status(), StatusCode::CREATED);
    body_json(response).await["id"]
        .as_str()
        .expect("缺少班级 id")
        .to_owned()
}

async fn create_lesson(
    app: &Router,
    token: &str,
    class_id: &str,
    lesson_date: &str,
    topic: &str,
) -> Response {
    app.clone()
        .oneshot(request(
            "POST",
            "/api/v1/lessons",
            Some(token),
            Some(json!({
                "classId": class_id,
                "lessonDate": lesson_date,
                "topic": topic,
            })),
        ))
        .await
        .expect("创建课次失败")
}

async fn list_lessons_page(app: &Router, token: &str, query: &str) -> Value {
    let response = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/api/v1/lessons?{query}"),
            Some(token),
            None,
        ))
        .await
        .expect("查询课次失败");
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

async fn wait_for_database_lock(pool: &PgPool) {
    for _ in 0..200 {
        let waiting: i64 = sqlx::query_scalar(
            "SELECT count(*)
             FROM pg_locks l
             JOIN pg_stat_activity a ON a.pid = l.pid
             WHERE NOT l.granted AND a.datname = current_database()",
        )
        .fetch_one(pool)
        .await
        .expect("查询锁等待失败");
        if waiting > 0 {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let stats: Vec<(String, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT state, wait_event_type, wait_event, left(query, 120)
         FROM pg_stat_activity
         WHERE datname = current_database()",
    )
    .fetch_all(pool)
    .await
    .expect("查询活动会话失败");
    panic!("创建请求未进入等待锁状态: {stats:?}");
}

#[sqlx::test(migrations = false)]
async fn lesson_create_and_cursor_pagination(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();
    let class_id = create_class(&app, token, "初二数学").await;

    for day in 1..=25 {
        let response = create_lesson(
            &app,
            token,
            &class_id,
            &format!("2026-01-{day:02}"),
            &format!("第 {day} 课"),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let created = body_json(response).await;
        assert_eq!(created["status"], "draft");
        assert_eq!(created["classId"], class_id);
    }

    let mut cursor: Option<String> = None;
    let mut ids = HashSet::new();
    let mut pages = 0;
    let mut previous_date: Option<String> = None;
    loop {
        let query = match &cursor {
            Some(cursor) => format!("limit=10&cursor={cursor}"),
            None => "limit=10".to_owned(),
        };
        let page = list_lessons_page(&app, token, &query).await;
        let items = page["items"].as_array().expect("items 不是数组");
        assert!(items.len() <= 10);
        for item in items {
            let id = item["id"].as_str().expect("缺少课次 id");
            assert!(ids.insert(id.to_owned()), "游标翻页出现重复课次");
            let date = item["lessonDate"].as_str().expect("缺少课次日期");
            if let Some(previous) = &previous_date {
                assert!(previous.as_str() >= date, "课次未按日期倒序排列");
            }
            previous_date = Some(date.to_owned());
        }
        pages += 1;
        cursor = page["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(pages, 3);
    assert_eq!(ids.len(), 25);

    let default_page = list_lessons_page(&app, token, "").await;
    assert_eq!(
        default_page["items"]
            .as_array()
            .expect("items 不是数组")
            .len(),
        20
    );
    assert!(default_page["nextCursor"].is_string());

    let filtered = list_lessons_page(&app, token, "from=2026-01-05&to=2026-01-14").await;
    assert_eq!(
        filtered["items"].as_array().expect("items 不是数组").len(),
        10
    );
    assert_eq!(filtered["nextCursor"], Value::Null);

    let upper = list_lessons_page(&app, token, "limit=100").await;
    assert_eq!(upper["items"].as_array().expect("items 不是数组").len(), 25);
    assert!(upper["nextCursor"].is_null());

    let exact = list_lessons_page(&app, token, "limit=25").await;
    assert_eq!(exact["items"].as_array().expect("items 不是数组").len(), 25);
    assert!(exact["nextCursor"].is_null(), "整页时不应返回下一页游标");
}

#[sqlx::test(migrations = false)]
async fn lesson_validation_and_tenant_isolation(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let owner = create_account(&admin, "owner@example.com").await;
    let other = create_account(&admin, "other@example.com").await;
    let token = owner.session_token.as_str();
    let class_id = create_class(&app, token, "班级").await;
    let foreign_class = create_class(&app, other.session_token.as_str(), "他人班级").await;

    let invalid_date = create_lesson(&app, token, &class_id, "2026-13-01", "").await;
    assert_eq!(invalid_date.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let invalid_date = body_json(invalid_date).await;
    assert_eq!(invalid_date["errors"][0]["field"], "lessonDate");

    let foreign = create_lesson(&app, token, &foreign_class, "2026-02-01", "").await;
    assert_eq!(foreign.status(), StatusCode::NOT_FOUND);

    let archived_class = create_class(&app, token, "归档班级").await;
    let archived = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/v1/classes/{archived_class}/archive"),
            Some(token),
            None,
        ))
        .await
        .expect("归档班级失败");
    assert_eq!(archived.status(), StatusCode::NO_CONTENT);
    let archived_lesson = create_lesson(&app, token, &archived_class, "2026-02-01", "").await;
    assert_eq!(
        archived_lesson.status(),
        StatusCode::NOT_FOUND,
        "归档班级不应接受新课次"
    );

    let too_long = "a".repeat(201);
    let invalid_topic = create_lesson(&app, token, &class_id, "2026-02-01", &too_long).await;
    assert_eq!(invalid_topic.status(), StatusCode::UNPROCESSABLE_ENTITY);

    for query in [
        "from=2026-02-02&to=2026-02-01",
        "limit=0",
        "limit=101",
        "cursor=eA",
    ] {
        let response = app
            .clone()
            .oneshot(request(
                "GET",
                &format!("/api/v1/lessons?{query}"),
                Some(token),
                None,
            ))
            .await
            .expect("校验请求失败");
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "查询 {query} 应返回 422"
        );
    }

    let unauthenticated = app
        .clone()
        .oneshot(request("GET", "/api/v1/lessons", None, None))
        .await
        .expect("未认证请求失败");
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

    let created = create_lesson(&app, token, &class_id, "2026-02-01", "课次").await;
    assert_eq!(created.status(), StatusCode::CREATED);
    let lesson_id = body_json(created).await["id"]
        .as_str()
        .expect("缺少课次 id")
        .to_owned();

    let foreign_lesson = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/v1/lessons/{lesson_id}/generations"),
            Some(other.session_token.as_str()),
            None,
        ))
        .await
        .expect("跨租户课次请求失败");
    assert_eq!(foreign_lesson.status(), StatusCode::NOT_FOUND);

    let foreign_list = list_lessons_page(&app, other.session_token.as_str(), "").await;
    assert_eq!(foreign_list["items"], json!([]));
}

#[sqlx::test(migrations = false)]
async fn concurrent_lesson_creation_paginates_without_duplicates(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str().to_owned();
    let class_id = create_class(&app, token.as_str(), "并发班级").await;

    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..30 {
        let app = app.clone();
        let token = token.clone();
        let class_id = class_id.clone();
        tasks.spawn(async move {
            let response = app
                .oneshot(request(
                    "POST",
                    "/api/v1/lessons",
                    Some(token.as_str()),
                    Some(json!({
                        "classId": class_id,
                        "lessonDate": "2026-03-01",
                        "topic": format!("并发 {index}"),
                    })),
                ))
                .await
                .expect("并发创建失败");
            assert_eq!(response.status(), StatusCode::CREATED);
        });
    }
    while let Some(result) = tasks.join_next().await {
        result.expect("并发任务失败");
    }

    let mut cursor: Option<String> = None;
    let mut ids = HashSet::new();
    loop {
        let query = match &cursor {
            Some(cursor) => format!("limit=7&cursor={cursor}"),
            None => "limit=7".to_owned(),
        };
        let page = list_lessons_page(&app, token.as_str(), &query).await;
        for item in page["items"].as_array().expect("items 不是数组") {
            let id = item["id"].as_str().expect("缺少课次 id");
            assert!(ids.insert(id.to_owned()), "并发新增后出现重复课次");
        }
        cursor = page["nextCursor"].as_str().map(str::to_owned);
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(ids.len(), 30);
}

#[sqlx::test(migrations = false)]
async fn lesson_create_returns_404_when_class_is_deleted_concurrently(pool: PgPool) {
    let (admin, app) = setup(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str().to_owned();
    let class_id = create_class(&app, token.as_str(), "并发删除班级").await;
    let class_uuid: Uuid = class_id.parse().expect("班级 id 非法");

    let mut delete_transaction = pool.begin().await.expect("开启删除事务失败");
    sqlx::query("SELECT id FROM classes WHERE id = $1 FOR UPDATE")
        .bind(class_uuid)
        .execute(&mut *delete_transaction)
        .await
        .expect("锁定班级失败");
    sqlx::query("DELETE FROM classes WHERE id = $1")
        .bind(class_uuid)
        .execute(&mut *delete_transaction)
        .await
        .expect("删除班级失败");

    let request_app = app.clone();
    let request_token = token.clone();
    let request_class = class_id.clone();
    let create_task = tokio::spawn(async move {
        request_app
            .oneshot(request(
                "POST",
                "/api/v1/lessons",
                Some(request_token.as_str()),
                Some(json!({
                    "classId": request_class,
                    "lessonDate": "2026-04-01",
                })),
            ))
            .await
            .expect("创建课次请求失败")
    });

    wait_for_database_lock(&pool).await;
    delete_transaction.commit().await.expect("提交删除事务失败");

    let response = create_task.await.expect("创建课次任务失败");
    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "班级并发删除时应返回 404，而不是外键错误导致的 500"
    );
}
