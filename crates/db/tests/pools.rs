//! 双连接池与迁移 runner 的集成测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use db::{Db, DbConfig};
use sqlx::PgPool;
use uuid::Uuid;

fn db_from_pool(pool: &PgPool) -> Db {
    let options = pool.connect_options().as_ref().clone();
    Db::new(&DbConfig::from_options(options.clone(), options))
}

#[sqlx::test(migrations = false)]
async fn migrations_can_run_twice(pool: PgPool) {
    db::run_migrations(&pool).await.expect("首次迁移失败");
    db::run_migrations(&pool).await.expect("重复迁移失败");
}

#[sqlx::test(migrations = false)]
async fn tenant_transaction_sets_app_tenant_id(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let db = db_from_pool(&pool);
    let tenant_id = Uuid::new_v4();

    let mut transaction = db
        .tenant()
        .begin(tenant_id)
        .await
        .expect("开启租户事务失败");
    let current: String = sqlx::query_scalar("SELECT current_setting('app.tenant_id', true)")
        .fetch_one(&mut **transaction.transaction_mut())
        .await
        .expect("读取租户上下文失败");

    assert_eq!(current, tenant_id.to_string());
    transaction.commit().await.expect("提交租户事务失败");
}

#[sqlx::test(migrations = false)]
async fn health_check_reaches_both_pools(pool: PgPool) {
    let db = db_from_pool(&pool);
    db.health_check().await.expect("健康检查失败");
}

#[sqlx::test(migrations = false)]
async fn admin_pool_can_read_users(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("test@example.com")
        .bind("not-a-real-hash")
        .execute(&pool)
        .await
        .expect("插入测试用户失败");

    let db = db_from_pool(&pool);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(db.admin().pool())
        .await
        .expect("管理池读取 users 失败");

    assert_eq!(count, 1);
    assert_eq!(db.tenant().max_connections(), 10);
    assert_eq!(db.admin().max_connections(), 5);
}
