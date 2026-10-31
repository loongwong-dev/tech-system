//! 批量生成领域集成测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use db::{Db, DbConfig};
use domain::generate_batch;
use sqlx::PgPool;
use uuid::Uuid;

async fn seed_lesson(pool: &PgPool, tenant_id: Uuid) -> Uuid {
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

    for index in 0..2 {
        let student_id = Uuid::new_v4();
        sqlx::query("INSERT INTO students (id, tenant_id, name) VALUES ($1, $2, $3)")
            .bind(student_id)
            .bind(tenant_id)
            .bind(format!("学员 {index}"))
            .execute(pool)
            .await
            .expect("创建学员失败");
        let id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO feedbacks (id, tenant_id, lesson_id, student_id, notes, knowledge_tags)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(lesson_id)
        .bind(student_id)
        .bind(format!("第 {index} 份教师原文"))
        .bind(vec!["专注投入".to_owned()])
        .execute(pool)
        .await
        .expect("创建反馈失败");
    }
    lesson_id
}

#[sqlx::test(migrations = false)]
async fn batch_generates_template_text_for_all_feedback(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_id)
        .bind("租户")
        .bind(format!("tenant-{}", tenant_id.simple()))
        .execute(&pool)
        .await
        .expect("创建租户失败");
    let lesson_id = seed_lesson(&pool, tenant_id).await;

    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    let generated = generate_batch(db.tenant(), tenant_id, lesson_id)
        .await
        .expect("批量生成失败");
    assert_eq!(generated, 2);

    let rows: Vec<(String, String, Option<serde_json::Value>)> = sqlx::query_as(
        "SELECT status, notes, rendered FROM feedbacks WHERE lesson_id = $1 ORDER BY created_at",
    )
    .bind(lesson_id)
    .fetch_all(&pool)
    .await
    .expect("读取反馈失败");
    assert_eq!(rows.len(), 2);
    for (status, notes, rendered) in rows {
        assert_eq!(status, "generated");
        assert!(notes.starts_with("第 "));
        assert!(
            rendered
                .as_ref()
                .and_then(|value| value.get("text"))
                .and_then(serde_json::Value::as_str)
                .is_some_and(|text| text.contains("建议"))
        );
    }
}

#[sqlx::test(migrations = false)]
async fn batch_rejects_lesson_without_feedback(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_id)
        .bind("租户")
        .bind(format!("tenant-{}", tenant_id.simple()))
        .execute(&pool)
        .await
        .expect("创建租户失败");
    let class_id = Uuid::new_v4();
    sqlx::query("INSERT INTO classes (id, tenant_id, name) VALUES ($1, $2, $3)")
        .bind(class_id)
        .bind(tenant_id)
        .bind("班级")
        .execute(&pool)
        .await
        .expect("创建班级失败");
    let lesson_id = Uuid::new_v4();
    sqlx::query("INSERT INTO lessons (id, tenant_id, class_id, lesson_date) VALUES ($1, $2, $3, CURRENT_DATE)")
        .bind(lesson_id)
        .bind(tenant_id)
        .bind(class_id)
        .execute(&pool)
        .await
        .expect("创建课次失败");

    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    let error = generate_batch(db.tenant(), tenant_id, lesson_id)
        .await
        .expect_err("空课次应失败");
    assert!(matches!(error, domain::GenerationError::LessonEmpty));
}
