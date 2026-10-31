//! 注册流程 SQL 访问。

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// 创建租户，返回主键。
pub(crate) async fn create_tenant(
    transaction: &mut Transaction<'_, Postgres>,
    name: &str,
    slug: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO tenants (name, slug)
        VALUES ($1, $2)
        RETURNING id
        "#,
        name,
        slug
    )
    .fetch_one(transaction.as_mut())
    .await
}

/// 创建用户，返回主键。
pub(crate) async fn create_user(
    transaction: &mut Transaction<'_, Postgres>,
    email: &str,
    password_hash: &str,
) -> Result<Uuid, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO users (email, password_hash)
        VALUES ($1, $2)
        RETURNING id
        "#,
        email,
        password_hash
    )
    .fetch_one(transaction.as_mut())
    .await
}

/// 将用户加入租户并设为 owner。
pub(crate) async fn create_membership(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO memberships (tenant_id, user_id, role)
        VALUES ($1, $2, 'owner')
        "#,
        tenant_id,
        user_id
    )
    .execute(transaction.as_mut())
    .await?;
    Ok(())
}
