//! 任务队列仓储：入队、领取、回收、完成、失败与取消。
//!
//! 所有领取语句都是单条 `UPDATE ... FROM (SELECT ... FOR UPDATE SKIP LOCKED)`，
//! 避免先查后写产生重复领取。队列本身跨租户读取，调用方必须使用管理池。

use std::time::Duration;

use sqlx::{Executor, PgPool, Postgres};
use uuid::Uuid;

use crate::error::JobError;
use crate::model::Job;
use crate::model::NewJob;

/// `last_error` 的最大字符数，防止异常堆栈无限膨胀。
const MAX_ERROR_CHARS: usize = 2_000;
/// 单任务最大尝试次数上限，防止误配置导致长期占用。
const MAX_ATTEMPTS_LIMIT: i32 = 100;
/// 回收扫描的单批上限。
const RECLAIM_BATCH_LIMIT: i64 = 100;

/// 事务内或连接池上的入队。
///
/// `dedupe_key` 只在 `pending` / `running` 状态下唯一；重复入队返回已存在的
/// 活跃任务，而不是新建第二条。调用方可以传 `&mut *transaction`，让任务与
/// 业务写入同事务提交。
///
/// # Errors
/// 参数为空、尝试次数越界或数据库写入失败时返回 [`JobError`]。
pub async fn enqueue<'e, E>(executor: E, input: NewJob) -> Result<Job, JobError>
where
    E: Executor<'e, Database = Postgres>,
{
    validate_input(&input)?;
    let tenant_id = input.tenant_id;
    let kind = input.kind;
    let dedupe_key = input.dedupe_key;
    let payload = input.payload;
    let max_attempts = input.max_attempts;
    let run_at = input.run_at;

    sqlx::query_as!(
        Job,
        r#"
        INSERT INTO jobs (tenant_id, kind, dedupe_key, payload, max_attempts, run_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT (tenant_id, kind, dedupe_key)
          WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'running')
        DO UPDATE SET dedupe_key = EXCLUDED.dedupe_key
        RETURNING
          id,
          tenant_id,
          kind,
          dedupe_key,
          payload,
          status,
          attempts,
          max_attempts,
          run_at,
          locked_by,
          locked_until,
          last_error,
          created_at,
          updated_at,
          finished_at
        "#,
        tenant_id,
        kind,
        dedupe_key.as_deref(),
        payload,
        max_attempts,
        run_at,
    )
    .fetch_one(executor)
    .await
    .map_err(|source| JobError::Database { source })
}

/// 领取一条可运行任务并写入租约。
///
/// 只领取 `pending` 且 `run_at <= now()` 的任务；过期租约必须先由
/// [`reclaim_expired`] 回收。领取会原子地把 `attempts` 加一。
///
/// # Errors
/// worker 标识为空、租约为零或数据库操作失败时返回 [`JobError`]。
pub async fn claim(
    pool: &PgPool,
    worker_id: &str,
    lease: Duration,
) -> Result<Option<Job>, JobError> {
    if worker_id.trim().is_empty() {
        return Err(JobError::InvalidInput {
            reason: "worker_id 不能为空",
        });
    }
    if lease.is_zero() {
        return Err(JobError::InvalidInput {
            reason: "租约必须大于 0",
        });
    }
    let lease_seconds = lease.as_secs_f64();

    sqlx::query_as!(
        Job,
        r#"
        WITH picked AS (
          SELECT id
          FROM jobs
          WHERE status = 'pending'
            AND run_at <= now()
            AND attempts < max_attempts
          ORDER BY run_at, created_at, id
          FOR UPDATE SKIP LOCKED
          LIMIT 1
        )
        UPDATE jobs
        SET status = 'running',
            attempts = attempts + 1,
            locked_by = $1,
            locked_until = now() + make_interval(secs => $2),
            updated_at = now()
        FROM picked
        WHERE jobs.id = picked.id
        RETURNING
          jobs.id AS id,
          jobs.tenant_id AS tenant_id,
          jobs.kind AS kind,
          jobs.dedupe_key AS dedupe_key,
          jobs.payload AS payload,
          jobs.status AS status,
          jobs.attempts AS attempts,
          jobs.max_attempts AS max_attempts,
          jobs.run_at AS run_at,
          jobs.locked_by AS locked_by,
          jobs.locked_until AS locked_until,
          jobs.last_error AS last_error,
          jobs.created_at AS created_at,
          jobs.updated_at AS updated_at,
          jobs.finished_at AS finished_at
        "#,
        worker_id,
        lease_seconds,
    )
    .fetch_optional(pool)
    .await
    .map_err(|source| JobError::Database { source })
}

/// 回收过期租约。
///
/// 未超过最大尝试次数的任务回到 `pending` 并立即重新可领取；超过上限的
/// 任务进入 `dead`。返回本批实际回收的任务数。
///
/// # Errors
/// 数据库操作失败时返回 [`JobError`]。
pub async fn reclaim_expired(pool: &PgPool, limit: i64) -> Result<u64, JobError> {
    if limit <= 0 {
        return Err(JobError::InvalidInput {
            reason: "回收批量必须大于 0",
        });
    }
    let limit = limit.min(RECLAIM_BATCH_LIMIT);

    let result = sqlx::query!(
        r#"
        WITH expired AS (
          SELECT id
          FROM jobs
          WHERE status = 'running'
            AND locked_until <= now()
          ORDER BY locked_until
          FOR UPDATE SKIP LOCKED
          LIMIT $1
        )
        UPDATE jobs
        SET status = CASE
                WHEN jobs.attempts >= jobs.max_attempts THEN 'dead'
                ELSE 'pending'
            END,
            run_at = CASE
                WHEN jobs.attempts >= jobs.max_attempts THEN jobs.run_at
                ELSE now()
            END,
            finished_at = CASE
                WHEN jobs.attempts >= jobs.max_attempts THEN now()
                ELSE NULL
            END,
            last_error = COALESCE(jobs.last_error, '租约到期，worker 可能已崩溃'),
            locked_by = NULL,
            locked_until = NULL,
            updated_at = now()
        FROM expired
        WHERE jobs.id = expired.id
        "#,
        limit,
    )
    .execute(pool)
    .await
    .map_err(|source| JobError::Database { source })?;

    Ok(result.rows_affected())
}

/// 将任务标记为成功。
///
/// 只有仍由 `worker_id` 持有租约的运行中任务会被更新；已取消或已被回收的
/// 任务返回 `false`，避免迟到结果覆盖新状态。
///
/// # Errors
/// 数据库操作失败时返回 [`JobError`]。
pub async fn complete(pool: &PgPool, job_id: Uuid, worker_id: &str) -> Result<bool, JobError> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'succeeded',
            finished_at = now(),
            locked_by = NULL,
            locked_until = NULL,
            updated_at = now()
        WHERE id = $1
          AND status = 'running'
          AND locked_by = $2
        "#,
        job_id,
        worker_id,
    )
    .execute(pool)
    .await
    .map_err(|source| JobError::Database { source })?;

    Ok(result.rows_affected() == 1)
}

/// 记录任务失败并按退避策略重试。
///
/// 未超过最大尝试次数时回到 `pending`，`run_at` 推迟为指数退避加抖动；
/// 超过上限时进入 `dead` 并保留 `last_error`。只有仍由 `worker_id` 持有
/// 租约的运行中任务会被更新。
///
/// # Errors
/// 数据库操作失败时返回 [`JobError`]。
pub async fn fail(
    pool: &PgPool,
    job_id: Uuid,
    worker_id: &str,
    error: &str,
) -> Result<bool, JobError> {
    let message = truncate_error(error);
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = CASE
                WHEN attempts >= max_attempts THEN 'dead'
                ELSE 'pending'
            END,
            run_at = CASE
                WHEN attempts >= max_attempts THEN run_at
                ELSE now() + make_interval(
                    secs => LEAST(300.0, power(2.0, attempts)) * (0.75 + random() * 0.5)
                )
            END,
            finished_at = CASE
                WHEN attempts >= max_attempts THEN now()
                ELSE NULL
            END,
            last_error = $3,
            locked_by = NULL,
            locked_until = NULL,
            updated_at = now()
        WHERE id = $1
          AND status = 'running'
          AND locked_by = $2
        "#,
        job_id,
        worker_id,
        message,
    )
    .execute(pool)
    .await
    .map_err(|source| JobError::Database { source })?;

    Ok(result.rows_affected() == 1)
}

/// 取消租户的待执行或运行中任务。
///
/// 运行中的任务无法强制中止；worker 完成后的写回会因为状态已不是
/// `running` 而失败。返回是否确实取消了任务。
///
/// # Errors
/// 数据库操作失败时返回 [`JobError`]。
pub async fn cancel(pool: &PgPool, tenant_id: Uuid, job_id: Uuid) -> Result<bool, JobError> {
    let result = sqlx::query!(
        r#"
        UPDATE jobs
        SET status = 'cancelled',
            finished_at = now(),
            locked_by = NULL,
            locked_until = NULL,
            updated_at = now()
        WHERE id = $1
          AND tenant_id = $2
          AND status IN ('pending', 'running')
        "#,
        job_id,
        tenant_id,
    )
    .execute(pool)
    .await
    .map_err(|source| JobError::Database { source })?;

    Ok(result.rows_affected() == 1)
}

/// 按租户读取任务。
///
/// 使用显式 `tenant_id` 过滤，配合 RLS 防止跨租户读取；管理池调用时也不会
/// 返回其他租户的数据。
///
/// # Errors
/// 数据库操作失败时返回 [`JobError`]。
pub async fn get(pool: &PgPool, tenant_id: Uuid, job_id: Uuid) -> Result<Option<Job>, JobError> {
    sqlx::query_as!(
        Job,
        r#"
        SELECT
          id,
          tenant_id,
          kind,
          dedupe_key,
          payload,
          status,
          attempts,
          max_attempts,
          run_at,
          locked_by,
          locked_until,
          last_error,
          created_at,
          updated_at,
          finished_at
        FROM jobs
        WHERE id = $1 AND tenant_id = $2
        "#,
        job_id,
        tenant_id,
    )
    .fetch_optional(pool)
    .await
    .map_err(|source| JobError::Database { source })
}

fn validate_input(input: &NewJob) -> Result<(), JobError> {
    if input.kind.trim().is_empty() {
        return Err(JobError::InvalidInput {
            reason: "kind 不能为空",
        });
    }
    if input.max_attempts <= 0 || input.max_attempts > MAX_ATTEMPTS_LIMIT {
        return Err(JobError::InvalidInput {
            reason: "max_attempts 必须在 1..=100 之间",
        });
    }
    if input
        .dedupe_key
        .as_deref()
        .is_some_and(|key| key.trim().is_empty())
    {
        return Err(JobError::InvalidInput {
            reason: "dedupe_key 不能为空字符串",
        });
    }
    Ok(())
}

fn truncate_error(error: &str) -> String {
    let message: String = error.chars().take(MAX_ERROR_CHARS).collect();
    if message.trim().is_empty() {
        "任务执行失败".to_owned()
    } else {
        message
    }
}
