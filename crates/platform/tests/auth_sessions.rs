//! 登录会话校验、滑动续期与吊销。

// 测试允许 expect：失败信息用于定位断言；生产路径仍由 lint 禁止。
#![allow(clippy::expect_used, clippy::unwrap_used)]

use chrono::{TimeDelta, Utc};
use db::{AdminPool, Db, DbConfig};
use platform::{
    RegistrationInput, SessionError, generate_invite_code, register, revoke_session,
    validate_session,
};
use sha2::{Digest, Sha256};
use sqlx::PgPool;

async fn admin_pool(pool: &PgPool) -> AdminPool {
    db::run_migrations(pool).await.expect("迁移失败");
    let options = pool.connect_options().as_ref().clone();
    let db = Db::new(&DbConfig::from_options(options.clone(), options));
    db.admin().clone()
}

async fn create_account(admin: &AdminPool, email: &str) -> platform::RegisteredAccount {
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

async fn session_expiry(pool: &PgPool, token: &str) -> chrono::DateTime<Utc> {
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query_scalar("SELECT expires_at FROM auth_sessions WHERE token_hash = $1")
        .bind(hash)
        .fetch_one(pool)
        .await
        .expect("读取会话失败")
}

#[sqlx::test(migrations = false)]
async fn validates_session_and_renews_at_most_hourly(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();

    let context = validate_session(&admin, token).await.expect("会话校验失败");
    assert_eq!(context.user_id, account.user_id);
    assert_eq!(context.tenant_id, Some(account.tenant_id));

    let aged_expiry = Utc::now() + TimeDelta::days(29) - TimeDelta::hours(1);
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    sqlx::query("UPDATE auth_sessions SET expires_at = $1 WHERE token_hash = $2")
        .bind(aged_expiry)
        .bind(&hash)
        .execute(&pool)
        .await
        .expect("调整会话时间失败");

    validate_session(&admin, token).await.expect("续期校验失败");
    let renewed = session_expiry(&pool, token).await;
    assert!(renewed > aged_expiry);

    validate_session(&admin, token)
        .await
        .expect("第二次校验失败");
    assert_eq!(session_expiry(&pool, token).await, renewed);
}

#[sqlx::test(migrations = false)]
async fn revoked_session_is_expired(pool: PgPool) {
    let admin = admin_pool(&pool).await;
    let account = create_account(&admin, "teacher@example.com").await;
    let token = account.session_token.as_str();

    revoke_session(&admin, token).await.expect("吊销失败");
    assert!(matches!(
        validate_session(&admin, token).await,
        Err(SessionError::Expired)
    ));
    assert!(matches!(
        revoke_session(&admin, token).await,
        Err(SessionError::Expired)
    ));
}
