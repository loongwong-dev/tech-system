//! 业务表 RLS 覆盖与跨租户隔离测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use db::{Db, DbConfig};
use sqlx::{PgPool, Postgres, Transaction};
use tokio::sync::Mutex;
use uuid::Uuid;

const BOOTSTRAP_SQL: &str = include_str!("../../../deploy/postgres/bootstrap.sql");

static BOOTSTRAP_LOCK: Mutex<()> = Mutex::const_new(());

const BUSINESS_TABLES: [&str; 10] = [
    "students",
    "classes",
    "lessons",
    "feedbacks",
    "share_artifacts",
    "usage_counters",
    "entitlements",
    "feedback_templates",
    "audit_logs",
    "jobs",
];

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
}

async fn act_as_tenant(transaction: &mut Transaction<'_, Postgres>) {
    sqlx::query("SET LOCAL ROLE app_tenant")
        .execute(&mut **transaction)
        .await
        .expect("切换到租户角色失败");
}

fn database(pool: &PgPool) -> Db {
    let options = pool.connect_options().as_ref().clone();
    Db::new(&DbConfig::from_options(options.clone(), options))
}

fn names(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_owned()).collect()
}

#[sqlx::test(migrations = false)]
async fn business_tables_enable_force_rls_and_have_policies(pool: PgPool) {
    bootstrap(&pool).await;

    let rows: Vec<(String, bool, bool, i64)> = sqlx::query_as(
        r"
        SELECT c.relname,
               c.relrowsecurity,
               c.relforcerowsecurity,
               (SELECT count(*) FROM pg_policy p WHERE p.polrelid = c.oid)::bigint
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relkind = 'r'
          AND c.relname = ANY($1)
        ",
    )
    .bind(names(&BUSINESS_TABLES))
    .fetch_all(&pool)
    .await
    .expect("读取 RLS 元数据失败");

    assert_eq!(rows.len(), BUSINESS_TABLES.len());
    for (table, enabled, forced, policies) in rows {
        assert!(enabled, "{table}: RLS 未启用");
        assert!(forced, "{table}: FORCE RLS 未启用");
        assert!(policies > 0, "{table}: 缺少策略");
    }

    let exceptions: Vec<(String, bool, bool)> = sqlx::query_as(
        r"
        SELECT c.relname, c.relrowsecurity, c.relforcerowsecurity
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = 'public'
          AND c.relkind = 'r'
          AND c.relname = ANY($1)
        ",
    )
    .bind(names(&EXCEPTION_TABLES))
    .fetch_all(&pool)
    .await
    .expect("读取例外表元数据失败");

    assert_eq!(exceptions.len(), EXCEPTION_TABLES.len());
    for (table, enabled, forced) in exceptions {
        assert!(!enabled, "{table}: 例外表不应启用 RLS");
        assert!(!forced, "{table}: 例外表不应 FORCE RLS");
    }
}

#[sqlx::test(migrations = false)]
async fn tenant_context_isolates_rows_and_blocks_cross_tenant_insert(pool: PgPool) {
    bootstrap(&pool).await;
    let db = database(&pool);
    let tenant_a = Uuid::new_v4();
    let tenant_b = Uuid::new_v4();

    for (tenant_id, name) in [(tenant_a, "tenant-a"), (tenant_b, "tenant-b")] {
        sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, $2, $3)")
            .bind(tenant_id)
            .bind(name)
            .bind(format!("{name}-{}", tenant_id.simple()))
            .execute(db.admin().pool())
            .await
            .expect("创建租户失败");
    }

    for (tenant_id, student) in [(tenant_a, "学员 A"), (tenant_b, "学员 B")] {
        let mut transaction = db
            .tenant()
            .begin(tenant_id)
            .await
            .expect("开启租户事务失败");
        act_as_tenant(transaction.transaction_mut()).await;
        sqlx::query("INSERT INTO students (tenant_id, name) VALUES ($1, $2)")
            .bind(tenant_id)
            .bind(student)
            .execute(&mut **transaction.transaction_mut())
            .await
            .expect("写入学员失败");
        transaction.commit().await.expect("提交失败");
    }

    let mut transaction = db.tenant().begin(tenant_a).await.expect("开启租户事务失败");
    act_as_tenant(transaction.transaction_mut()).await;
    let visible: Vec<String> = sqlx::query_scalar("SELECT name FROM students ORDER BY name")
        .fetch_all(&mut **transaction.transaction_mut())
        .await
        .expect("读取学员失败");
    assert_eq!(visible, vec!["学员 A".to_owned()]);

    let hidden: i64 = sqlx::query_scalar("SELECT count(*) FROM students WHERE tenant_id = $1")
        .bind(tenant_b)
        .fetch_one(&mut **transaction.transaction_mut())
        .await
        .expect("读取跨租户学员失败");
    assert_eq!(hidden, 0);

    let mut no_context_transaction = db
        .admin()
        .pool()
        .begin()
        .await
        .expect("开启无上下文事务失败");
    act_as_tenant(&mut no_context_transaction).await;
    let no_context: i64 = sqlx::query_scalar("SELECT count(*) FROM students")
        .fetch_one(&mut *no_context_transaction)
        .await
        .expect("无租户上下文查询失败");
    assert_eq!(no_context, 0);

    let mut transaction = db.tenant().begin(tenant_a).await.expect("开启租户事务失败");
    act_as_tenant(transaction.transaction_mut()).await;
    let error = sqlx::query("INSERT INTO students (tenant_id, name) VALUES ($1, '越权')")
        .bind(tenant_b)
        .execute(&mut **transaction.transaction_mut())
        .await
        .expect_err("跨租户写入应失败");
    assert_eq!(
        error
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("42501")
    );
}

#[sqlx::test(migrations = false)]
async fn feedback_templates_read_builtin_but_only_write_own_tenant(pool: PgPool) {
    bootstrap(&pool).await;
    let db = database(&pool);
    let tenant_id = Uuid::new_v4();

    sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, 'tenant', $2)")
        .bind(tenant_id)
        .bind(format!("tenant-{}", tenant_id.simple()))
        .execute(db.admin().pool())
        .await
        .expect("创建租户失败");

    let mut transaction = db.admin().pool().begin().await.expect("开启事务失败");
    sqlx::query("SET LOCAL row_security = off")
        .execute(&mut *transaction)
        .await
        .expect("关闭 RLS 失败");
    sqlx::query(
        "INSERT INTO feedback_templates (tenant_id, name, body) VALUES (NULL, '内置', '正文')",
    )
    .execute(&mut *transaction)
    .await
    .expect("写入平台模板失败");
    transaction.commit().await.expect("提交失败");

    let mut transaction = db
        .tenant()
        .begin(tenant_id)
        .await
        .expect("开启租户事务失败");
    act_as_tenant(transaction.transaction_mut()).await;
    let builtin_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM feedback_templates WHERE tenant_id IS NULL")
            .fetch_one(&mut **transaction.transaction_mut())
            .await
            .expect("读取平台模板失败");
    assert_eq!(builtin_count, 1);

    sqlx::query(
        "INSERT INTO feedback_templates (tenant_id, name, body) VALUES ($1, '自有', '正文')",
    )
    .bind(tenant_id)
    .execute(&mut **transaction.transaction_mut())
    .await
    .expect("写入自有模板失败");

    let error = sqlx::query(
        "INSERT INTO feedback_templates (tenant_id, name, body) VALUES (NULL, '越权', '正文')",
    )
    .execute(&mut **transaction.transaction_mut())
    .await
    .expect_err("租户写入平台模板应失败");
    assert_eq!(
        error
            .as_database_error()
            .and_then(sqlx::error::DatabaseError::code)
            .as_deref(),
        Some("42501")
    );
}
