//! 双连接池、角色隔离与迁移 runner 的集成测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use db::{Db, DbConfig};
use sqlx::PgPool;
use tokio::sync::Mutex;
use uuid::Uuid;

/// 角色是集群级对象，同一测试进程内的 bootstrap 必须串行化，避免 CREATE/ALTER ROLE 竞态。
///
/// 该锁不跨进程；多个 integration test binary 并发操作同一 PostgreSQL 实例时，
/// 仍应使用独立实例或额外的外部串行化。
static BOOTSTRAP_LOCK: Mutex<()> = Mutex::const_new(());

const BOOTSTRAP_SQL: &str = include_str!("../../../deploy/postgres/bootstrap.sql");
const TENANT_PASSWORD: &str = "test-tenant-password";
const ADMIN_PASSWORD: &str = "test-admin-password";
const EXCEPTION_TABLES: [&str; 6] = [
    "users",
    "tenants",
    "memberships",
    "invite_codes",
    "platform_admins",
    "auth_sessions",
];

async fn bootstrap(pool: &PgPool) {
    let _guard = BOOTSTRAP_LOCK.lock().await;
    db::run_migrations(pool).await.expect("迁移失败");
    sqlx::raw_sql(BOOTSTRAP_SQL)
        .execute(pool)
        .await
        .expect("角色引导失败");
    sqlx::query("ALTER ROLE app_tenant PASSWORD 'test-tenant-password'")
        .execute(pool)
        .await
        .expect("设置租户角色密码失败");
    sqlx::query("ALTER ROLE app_admin PASSWORD 'test-admin-password'")
        .execute(pool)
        .await
        .expect("设置管理角色密码失败");
}

fn role_db(pool: &PgPool) -> Db {
    let base = pool.connect_options().as_ref().clone();
    let tenant = base
        .clone()
        .username("app_tenant")
        .password(TENANT_PASSWORD);
    let admin = base.username("app_admin").password(ADMIN_PASSWORD);
    Db::new(&DbConfig::from_options(tenant, admin))
}

fn assert_permission_denied(error: &sqlx::Error, context: &str) {
    assert_eq!(
        error
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("42501"),
        "{context}: {error}"
    );
}

#[sqlx::test(migrations = false)]
async fn migrations_can_run_twice(pool: PgPool) {
    db::run_migrations(&pool).await.expect("首次迁移失败");
    db::run_migrations(&pool).await.expect("重复迁移失败");
}

#[sqlx::test(migrations = false)]
async fn bootstrap_is_idempotent(pool: PgPool) {
    bootstrap(&pool).await;
    sqlx::raw_sql(BOOTSTRAP_SQL)
        .execute(&pool)
        .await
        .expect("重复角色引导失败");
}

#[sqlx::test(migrations = false)]
async fn tenant_transaction_sets_app_tenant_id(pool: PgPool) {
    bootstrap(&pool).await;
    let db = role_db(&pool);
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
    bootstrap(&pool).await;
    let db = role_db(&pool);
    db.health_check().await.expect("健康检查失败");
}

#[sqlx::test(migrations = false)]
async fn admin_pool_can_read_users(pool: PgPool) {
    bootstrap(&pool).await;
    sqlx::query("INSERT INTO users (email, password_hash) VALUES ($1, $2)")
        .bind("test@example.com")
        .bind("not-a-real-hash")
        .execute(&pool)
        .await
        .expect("插入测试用户失败");

    let db = role_db(&pool);
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(db.admin().pool())
        .await
        .expect("管理池读取 users 失败");

    assert_eq!(count, 1);
}

#[sqlx::test(migrations = false)]
async fn tenant_pool_cannot_read_exception_tables(pool: PgPool) {
    bootstrap(&pool).await;
    let db = role_db(&pool);

    for table in EXCEPTION_TABLES {
        let mut transaction = db
            .tenant()
            .begin(Uuid::new_v4())
            .await
            .expect("开启租户事务失败");
        // 表名来自编译期常量数组，不包含用户输入；AssertSqlSafe 仅用于拼接标识符。
        let query = format!("SELECT count(*) FROM {table}");
        let error = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(query))
            .fetch_one(&mut **transaction.transaction_mut())
            .await
            .expect_err("租户角色不应读取平台例外表");
        assert_permission_denied(&error, table);
    }
}

#[sqlx::test(migrations = false)]
async fn tenant_can_read_business_tables(pool: PgPool) {
    bootstrap(&pool).await;
    sqlx::query("CREATE TABLE business_probe (id int)")
        .execute(&pool)
        .await
        .expect("创建业务探针表失败");

    let db = role_db(&pool);
    let mut transaction = db
        .tenant()
        .begin(Uuid::new_v4())
        .await
        .expect("开启租户事务失败");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM business_probe")
        .fetch_one(&mut **transaction.transaction_mut())
        .await
        .expect("租户角色应能读取业务表");

    assert_eq!(count, 0);
}

#[sqlx::test(migrations = false)]
async fn migration_history_is_owner_only(pool: PgPool) {
    bootstrap(&pool).await;
    let db = role_db(&pool);

    let mut tenant_transaction = db
        .tenant()
        .begin(Uuid::new_v4())
        .await
        .expect("开启租户事务失败");
    let tenant_error = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(&mut **tenant_transaction.transaction_mut())
        .await
        .expect_err("租户角色不应读取迁移历史");
    assert_permission_denied(&tenant_error, "tenant/_sqlx_migrations");

    let admin_error = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM _sqlx_migrations")
        .fetch_one(db.admin().pool())
        .await
        .expect_err("管理角色不应读取迁移历史");
    assert_permission_denied(&admin_error, "admin/_sqlx_migrations");
}
