//! 配额摘要计算。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{Datelike, FixedOffset, TimeZone, Utc};
use db::{AdminPool, Db, DbConfig};
use platform::{RegistrationInput, generate_invite_code, quota_summary, register};
use sqlx::PgPool;

async fn admin_pool(pool: &PgPool) -> AdminPool {
    db::run_migrations(pool).await.expect("迁移失败");
    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    db.admin().clone()
}

async fn create_tenant(admin: &AdminPool) -> uuid::Uuid {
    let invite = generate_invite_code(admin, None, 1, None)
        .await
        .expect("生成邀请码失败")
        .code;
    register(
        admin,
        RegistrationInput {
            email: "teacher@example.com".to_owned(),
            password: "a-long-enough-password".to_owned(),
            invite_code: invite,
        },
    )
    .await
    .expect("注册失败")
    .tenant_id
}

#[sqlx::test(migrations = false)]
async fn adds_entitlements_and_reads_current_month_usage(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let tenant_id = create_tenant(&admin).await;

    let offset = FixedOffset::east_opt(8 * 3600).expect("固定时区有效");
    let now = Utc::now().with_timezone(&offset);
    let period = offset
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .expect("月初时间有效")
        .date_naive();

    sqlx::query(
        "INSERT INTO usage_counters (tenant_id, period, metric, used) VALUES ($1, $2, $3, $4)",
    )
    .bind(tenant_id)
    .bind(period)
    .bind("feedback_generated")
    .bind(5_i32)
    .execute(&pool)
    .await
    .expect("写入用量失败");
    sqlx::query(
        "INSERT INTO entitlements (tenant_id, addon_code, quota_delta) VALUES ($1, $2, $3)",
    )
    .bind(tenant_id)
    .bind("addon_200")
    .bind(200_i32)
    .execute(&pool)
    .await
    .expect("写入权益失败");

    let quota = quota_summary(&admin, tenant_id)
        .await
        .expect("读取配额失败");
    assert_eq!(quota.limit, 215);
    assert_eq!(quota.used, 5);
    assert_eq!(quota.remaining, 210);
    assert!(quota.reset_at > Utc::now());
}
