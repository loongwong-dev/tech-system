//! 邀请码生成、消耗与并发行为。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{TimeDelta, Utc};
use db::{AdminPool, Db, DbConfig};
use platform::{InviteCodeError, consume_invite_code, generate_invite_code};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

async fn admin_pool(pool: &PgPool) -> AdminPool {
    db::run_migrations(pool).await.expect("迁移失败");
    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    db.admin().clone()
}

#[sqlx::test(migrations = false)]
async fn generates_crockford_code_and_stores_only_hash(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let generated = generate_invite_code(&admin, Some("内测"), 3, None)
        .await
        .expect("生成邀请码失败");

    assert_eq!(generated.code.len(), 10);
    assert!(
        generated
            .code
            .bytes()
            .all(|byte| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&byte))
    );

    let (code_hash, note, max_uses, used_count): (Vec<u8>, Option<String>, i32, i32) =
        sqlx::query_as(
            "SELECT code_hash, note, max_uses, used_count FROM invite_codes WHERE id = $1",
        )
        .bind(generated.id)
        .fetch_one(&pool)
        .await
        .expect("读取邀请码失败");

    let expected_hash = Sha256::digest(generated.code.as_bytes()).to_vec();
    assert_eq!(code_hash, expected_hash);
    assert_eq!(note.as_deref(), Some("内测"));
    assert_eq!(max_uses, 3);
    assert_eq!(used_count, 0);
}

#[sqlx::test(migrations = false)]
async fn rejects_non_positive_max_uses(pool: PgPool) {
    let admin = admin_pool(&pool).await;

    for max_uses in [0, -1] {
        let result = generate_invite_code(&admin, None, max_uses, None).await;
        assert!(matches!(result, Err(InviteCodeError::InvalidParameters)));
    }
}

#[sqlx::test(migrations = false)]
async fn respects_max_uses_greater_than_one(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let generated = generate_invite_code(&admin, None, 3, None)
        .await
        .expect("生成邀请码失败");

    for _ in 0..3 {
        consume_invite_code(&admin, &generated.code)
            .await
            .expect("未超过 max_uses 时应成功");
    }

    assert!(matches!(
        consume_invite_code(&admin, &generated.code).await,
        Err(InviteCodeError::Invalid)
    ));
}

#[sqlx::test(migrations = false)]
async fn consumes_once_then_rejects_reuse(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let generated = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败");

    let consumed_id = consume_invite_code(&admin, &generated.code)
        .await
        .expect("首次消耗失败");
    assert_eq!(consumed_id, generated.id);

    let reused = consume_invite_code(&admin, &generated.code).await;
    assert!(matches!(reused, Err(InviteCodeError::Invalid)));
}

#[sqlx::test(migrations = false)]
async fn accepts_lowercase_and_separators(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let generated = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败");
    let formatted = format!(
        "{}-{}",
        generated.code[..5].to_lowercase(),
        generated.code[5..].to_lowercase()
    );

    let consumed = consume_invite_code(&admin, &formatted).await;
    assert_eq!(consumed.expect("规范化后消耗失败"), generated.id);
}

#[sqlx::test(migrations = false)]
async fn expired_used_and_missing_share_the_same_error(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let expired = generate_invite_code(&admin, None, 1, Some(Utc::now() - TimeDelta::days(1)))
        .await
        .expect("生成过期邀请码失败");
    assert!(matches!(
        consume_invite_code(&admin, &expired.code).await,
        Err(InviteCodeError::Invalid)
    ));

    let used = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败");
    consume_invite_code(&admin, &used.code)
        .await
        .expect("首次消耗失败");
    assert!(matches!(
        consume_invite_code(&admin, &used.code).await,
        Err(InviteCodeError::Invalid)
    ));

    assert!(matches!(
        consume_invite_code(&admin, "0000000000").await,
        Err(InviteCodeError::Invalid)
    ));
    assert!(matches!(
        consume_invite_code(&admin, "not-a-code").await,
        Err(InviteCodeError::Invalid)
    ));
}

#[sqlx::test(migrations = false)]
async fn concurrent_consumption_allows_exactly_one(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let generated = generate_invite_code(&admin, None, 1, None)
        .await
        .expect("生成邀请码失败");

    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let admin = admin.clone();
        let code = generated.code.clone();
        tasks.spawn(async move { consume_invite_code(&admin, &code).await });
    }

    let mut successes = 0;
    let mut invalid = 0;
    while let Some(result) = tasks.join_next().await {
        match result.expect("任务 panic") {
            Ok(id) => {
                assert_eq!(id, generated.id);
                successes += 1;
            }
            Err(InviteCodeError::Invalid) => invalid += 1,
            Err(error) => panic!("未预期错误: {error:?}"),
        }
    }

    assert_eq!(successes, 1);
    assert_eq!(invalid, 19);
}
