//! 队列并发、回收、死信、幂等、取消与优雅退出测试。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use jobs::{
    Handlers, Job, NewJob, STATUS_CANCELLED, STATUS_DEAD, STATUS_PENDING, STATUS_SUCCEEDED,
    WorkerConfig, cancel, claim, complete, enqueue, fail, get, reclaim_expired,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::sync::{Notify, watch};
use tokio::time::timeout;
use uuid::Uuid;

async fn seed_tenant(pool: &PgPool) -> Uuid {
    let tenant_id = Uuid::new_v4();
    sqlx::query("INSERT INTO tenants (id, name, slug) VALUES ($1, $2, $3)")
        .bind(tenant_id)
        .bind("测试租户")
        .bind(format!("tenant-{}", tenant_id.simple()))
        .execute(pool)
        .await
        .expect("创建租户失败");
    tenant_id
}

fn new_job(tenant_id: Uuid, kind: &str) -> NewJob {
    NewJob::new(tenant_id, kind, json!({"value": 1}))
}

#[sqlx::test(migrations = false)]
async fn enqueue_rolls_back_with_business_transaction(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;

    let mut transaction = pool.begin().await.expect("开启事务失败");
    enqueue(&mut *transaction, new_job(tenant_id, "transactional"))
        .await
        .expect("事务内入队失败");
    transaction.rollback().await.expect("回滚失败");

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .expect("统计任务失败");
    assert_eq!(count, 0, "业务回滚后任务不应残留");
}

#[sqlx::test(migrations = false)]
async fn concurrent_claimers_receive_each_job_once(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;
    for _ in 0..20 {
        enqueue(&pool, new_job(tenant_id, "noop"))
            .await
            .expect("入队失败");
    }

    let mut tasks = tokio::task::JoinSet::new();
    for index in 0..8 {
        let pool = pool.clone();
        tasks.spawn(async move {
            let worker_id = format!("worker-{index}");
            let mut claimed = Vec::new();
            while let Some(job) = claim(&pool, &worker_id, Duration::from_secs(30))
                .await
                .expect("领取失败")
            {
                complete(&pool, job.id, &worker_id)
                    .await
                    .expect("完成任务失败");
                claimed.push(job.id);
            }
            claimed
        });
    }

    let mut all = Vec::new();
    while let Some(result) = tasks.join_next().await {
        all.extend(result.expect("worker task panicked"));
    }

    let unique: HashSet<_> = all.iter().copied().collect();
    assert_eq!(all.len(), 20, "任务被领取次数不正确");
    assert_eq!(unique.len(), 20, "同一个任务被重复领取");

    let succeeded: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs WHERE status = $1")
        .bind(STATUS_SUCCEEDED)
        .fetch_one(&pool)
        .await
        .expect("统计成功任务失败");
    assert_eq!(succeeded, 20);
}

#[sqlx::test(migrations = false)]
async fn expired_lease_returns_job_to_pending_and_increments_attempts(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;
    let enqueued = enqueue(&pool, new_job(tenant_id, "noop"))
        .await
        .expect("入队失败");
    let claimed = claim(&pool, "worker-a", Duration::from_secs(30))
        .await
        .expect("领取失败")
        .expect("队列为空");
    assert_eq!(claimed.attempts, 1);

    // 直接改租约时间模拟进程崩溃，避免测试等待真实 30 秒。
    sqlx::query("UPDATE jobs SET locked_until = now() - interval '1 second' WHERE id = $1")
        .bind(claimed.id)
        .execute(&pool)
        .await
        .expect("模拟租约过期失败");

    let reclaimed = reclaim_expired(&pool, 100).await.expect("回收失败");
    assert_eq!(reclaimed, 1);

    let pending = get(&pool, tenant_id, enqueued.id)
        .await
        .expect("读取任务失败")
        .expect("任务不存在");
    assert_eq!(pending.status, STATUS_PENDING);
    assert_eq!(pending.attempts, 1);
    assert!(pending.locked_by.is_none());
    assert!(pending.locked_until.is_none());

    let reclaimed_job = claim(&pool, "worker-b", Duration::from_secs(30))
        .await
        .expect("再次领取失败")
        .expect("回收后的任务不可领取");
    assert_eq!(reclaimed_job.id, enqueued.id);
    assert_eq!(reclaimed_job.attempts, 2);
}

#[sqlx::test(migrations = false)]
async fn poison_job_moves_to_dead_and_does_not_block_other_jobs(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;
    let poison = enqueue(&pool, new_job(tenant_id, "poison").with_max_attempts(1))
        .await
        .expect("入队失败");
    let claimed = claim(&pool, "worker-a", Duration::from_secs(30))
        .await
        .expect("领取失败")
        .expect("队列为空");
    assert!(
        fail(&pool, claimed.id, "worker-a", "始终失败")
            .await
            .expect("记录失败失败")
    );

    let dead = get(&pool, tenant_id, poison.id)
        .await
        .expect("读取任务失败")
        .expect("任务不存在");
    assert_eq!(dead.status, STATUS_DEAD);
    assert_eq!(dead.last_error.as_deref(), Some("始终失败"));
    assert!(dead.finished_at.is_some());

    enqueue(&pool, new_job(tenant_id, "healthy"))
        .await
        .expect("入队失败");
    let healthy = claim(&pool, "worker-b", Duration::from_secs(30))
        .await
        .expect("领取失败")
        .expect("毒任务阻塞了其他任务");
    assert_eq!(healthy.kind, "healthy");
}

#[sqlx::test(migrations = false)]
async fn failed_job_retries_with_backoff_then_dies(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;
    let job = enqueue(&pool, new_job(tenant_id, "flaky").with_max_attempts(2))
        .await
        .expect("入队失败");
    let first = claim(&pool, "worker-a", Duration::from_secs(30))
        .await
        .expect("领取失败")
        .expect("队列为空");
    assert_eq!(first.attempts, 1);
    assert!(
        fail(&pool, first.id, "worker-a", "第一次失败")
            .await
            .expect("记录失败失败")
    );

    let retrying = get(&pool, tenant_id, job.id)
        .await
        .expect("读取任务失败")
        .expect("任务不存在");
    assert_eq!(retrying.status, STATUS_PENDING);
    assert_eq!(retrying.attempts, 1);
    assert!(retrying.run_at > chrono::Utc::now(), "退避未推迟 run_at");

    // 跳过真实退避等待，验证第二次失败进入死信。
    sqlx::query("UPDATE jobs SET run_at = now() - interval '1 second' WHERE id = $1")
        .bind(job.id)
        .execute(&pool)
        .await
        .expect("提前 run_at 失败");
    let second = claim(&pool, "worker-b", Duration::from_secs(30))
        .await
        .expect("领取失败")
        .expect("退避后的任务不可领取");
    assert_eq!(second.attempts, 2);
    assert!(
        fail(&pool, second.id, "worker-b", "第二次失败")
            .await
            .expect("记录失败失败")
    );

    let dead = get(&pool, tenant_id, job.id)
        .await
        .expect("读取任务失败")
        .expect("任务不存在");
    assert_eq!(dead.status, STATUS_DEAD);
    assert_eq!(dead.attempts, 2);
}

#[sqlx::test(migrations = false)]
async fn dedupe_key_returns_active_job_and_is_scoped_to_tenant(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_a = seed_tenant(&pool).await;
    let tenant_b = seed_tenant(&pool).await;

    let first = enqueue(
        &pool,
        new_job(tenant_a, "batch").with_dedupe_key("lesson-1"),
    )
    .await
    .expect("首次入队失败");
    let second = enqueue(
        &pool,
        new_job(tenant_a, "batch").with_dedupe_key("lesson-1"),
    )
    .await
    .expect("重复入队失败");
    assert_eq!(first.id, second.id);

    let other_tenant = enqueue(
        &pool,
        new_job(tenant_b, "batch").with_dedupe_key("lesson-1"),
    )
    .await
    .expect("其他租户入队失败");
    assert_ne!(first.id, other_tenant.id);

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM jobs")
        .fetch_one(&pool)
        .await
        .expect("统计任务失败");
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = false)]
async fn cancelled_job_is_not_claimable_and_cannot_be_completed(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;
    let pending = enqueue(&pool, new_job(tenant_id, "noop"))
        .await
        .expect("入队失败");
    assert!(
        cancel(&pool, tenant_id, pending.id)
            .await
            .expect("取消失败")
    );
    assert!(
        claim(&pool, "worker-a", Duration::from_secs(30))
            .await
            .expect("领取失败")
            .is_none()
    );

    let running = enqueue(&pool, new_job(tenant_id, "noop"))
        .await
        .expect("入队失败");
    let claimed = claim(&pool, "worker-a", Duration::from_secs(30))
        .await
        .expect("领取失败")
        .expect("队列为空");
    assert_eq!(claimed.id, running.id);
    assert!(
        cancel(&pool, tenant_id, running.id)
            .await
            .expect("取消运行中任务失败")
    );
    assert!(
        !complete(&pool, running.id, "worker-a")
            .await
            .expect("完成写回失败")
    );

    let cancelled = get(&pool, tenant_id, running.id)
        .await
        .expect("读取任务失败")
        .expect("任务不存在");
    assert_eq!(cancelled.status, STATUS_CANCELLED);
}

#[sqlx::test(migrations = false)]
async fn worker_finishes_current_job_before_shutdown(pool: PgPool) {
    db::run_migrations(&pool).await.expect("迁移失败");
    let tenant_id = seed_tenant(&pool).await;
    let started = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let mut handlers = Handlers::new();
    handlers.register("slow", {
        let started = Arc::clone(&started);
        let release = Arc::clone(&release);
        move |_job: &Job| {
            let started = Arc::clone(&started);
            let release = Arc::clone(&release);
            Box::pin(async move {
                started.notify_one();
                release.notified().await;
                Ok(())
            })
        }
    });

    let job = enqueue(&pool, new_job(tenant_id, "slow"))
        .await
        .expect("入队失败");
    let config = WorkerConfig::new(
        "test-worker",
        Duration::from_millis(10),
        Duration::from_secs(30),
        Duration::from_secs(30),
    );
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let mut worker =
        tokio::spawn(jobs::Worker::new(pool.clone(), config, handlers).run(shutdown_rx));

    timeout(Duration::from_secs(2), started.notified())
        .await
        .expect("handler 未启动");

    shutdown_tx.send(true).expect("发送关闭信号失败");
    assert!(
        timeout(Duration::from_millis(100), &mut worker)
            .await
            .is_err(),
        "worker 在任务完成前退出"
    );

    release.notify_one();
    timeout(Duration::from_secs(2), worker)
        .await
        .expect("worker 未退出")
        .expect("worker task panicked")
        .expect("worker 返回错误");

    let finished = get(&pool, tenant_id, job.id)
        .await
        .expect("读取任务失败")
        .expect("任务不存在");
    assert_eq!(finished.status, STATUS_SUCCEEDED);
}
