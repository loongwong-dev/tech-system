use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::health::ping;
use crate::{DbConfig, DbError};

/// 获取连接的最长等待时间，避免请求无限挂起。
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// 受 RLS 约束的租户连接池。
///
/// 不暴露原始连接池；租户查询只能通过 [`TenantPool::begin`] 获得已设置
/// `app.tenant_id` 的事务。
#[derive(Clone)]
pub struct TenantPool {
    pool: PgPool,
}

impl TenantPool {
    /// 开启租户事务并设置 `app.tenant_id`。
    ///
    /// 使用 `set_config(..., true)` 的事务局部语义；事务结束后设置自动失效，
    /// 连接回池不会残留上一个租户的身份。
    ///
    /// # Errors
    /// 开启事务或设置租户上下文失败时返回 [`DbError::TenantContext`]。
    pub async fn begin(&self, tenant_id: Uuid) -> Result<TenantTransaction, DbError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|source| DbError::TenantContext { source })?;
        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
            .bind(tenant_id.to_string())
            .execute(&mut *transaction)
            .await
            .map_err(|source| DbError::TenantContext { source })?;
        Ok(TenantTransaction { transaction })
    }

    /// 返回配置的连接上限。
    #[must_use]
    pub fn max_connections(&self) -> u32 {
        self.pool.options().get_max_connections()
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// 已设置租户上下文的事务。
pub struct TenantTransaction {
    transaction: Transaction<'static, Postgres>,
}

impl TenantTransaction {
    /// 返回底层事务，供仓储层执行查询。
    ///
    /// 调用方必须在同一个事务内执行租户查询，不得把连接取出事务复用。
    #[must_use]
    pub fn transaction_mut(&mut self) -> &mut Transaction<'static, Postgres> {
        &mut self.transaction
    }

    /// 提交事务。
    ///
    /// # Errors
    /// 提交失败时返回 [`DbError::Commit`]。
    pub async fn commit(self) -> Result<(), DbError> {
        self.transaction
            .commit()
            .await
            .map_err(|source| DbError::Commit { source })
    }
}

/// 管理连接池，可访问不启用 RLS 的例外表。
///
/// 只在管理端路由与管理类任务中使用；租户路由不得持有该类型。
#[derive(Clone)]
pub struct AdminPool {
    pool: PgPool,
}

impl AdminPool {
    /// 返回底层连接池。
    #[must_use]
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// 返回配置的连接上限。
    #[must_use]
    pub fn max_connections(&self) -> u32 {
        self.pool.options().get_max_connections()
    }

    /// 检查指定限定表是否存在。
    ///
    /// # Errors
    /// 查询失败时返回 [`DbError::Query`]。
    pub async fn table_exists(&self, qualified_name: &str) -> Result<bool, DbError> {
        sqlx::query_scalar::<_, bool>("SELECT to_regclass($1) IS NOT NULL")
            .bind(qualified_name)
            .fetch_one(&self.pool)
            .await
            .map_err(|source| DbError::Query { source })
    }
}

/// 双连接池容器。
#[derive(Clone)]
pub struct Db {
    tenant: TenantPool,
    admin: AdminPool,
}

impl Db {
    /// 构造双连接池。
    ///
    /// 连接延迟建立：构造不访问数据库，首次使用或 [`Db::health_check`] 时校验可用性。
    #[must_use]
    pub fn new(config: &DbConfig) -> Self {
        let tenant_pool = PgPoolOptions::new()
            .max_connections(config.tenant_max_connections())
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .connect_lazy_with(config.tenant_options().clone());
        let admin_pool = PgPoolOptions::new()
            .max_connections(config.admin_max_connections())
            .acquire_timeout(ACQUIRE_TIMEOUT)
            .connect_lazy_with(config.admin_options().clone());
        Self {
            tenant: TenantPool { pool: tenant_pool },
            admin: AdminPool { pool: admin_pool },
        }
    }

    /// 从环境变量构造双连接池。
    ///
    /// # Errors
    /// 缺少连接串环境变量或连接串非法时返回错误。
    pub fn from_env() -> Result<Self, DbError> {
        Ok(Self::new(&DbConfig::from_env()?))
    }

    /// 返回租户池。
    #[must_use]
    pub fn tenant(&self) -> &TenantPool {
        &self.tenant
    }

    /// 返回管理池。
    #[must_use]
    pub fn admin(&self) -> &AdminPool {
        &self.admin
    }

    /// 检查两个连接池是否可用。
    ///
    /// # Errors
    /// 任一连接池不可用时返回 [`DbError::Health`]。
    pub async fn health_check(&self) -> Result<(), DbError> {
        ping(self.tenant.pool(), "tenant").await?;
        ping(self.admin.pool(), "admin").await
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use std::time::Duration;

    use sqlx::postgres::PgPoolOptions;

    use super::*;

    #[sqlx::test]
    async fn tenant_context_is_cleared_after_commit(pool: PgPool) {
        // 单连接池保证提交后的查询拿到同一个物理连接，才能验证 SET LOCAL 已清除。
        let options = pool.connect_options().as_ref().clone();
        let single_connection_pool = PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy_with(options);
        let tenant_pool = TenantPool {
            pool: single_connection_pool,
        };
        let tenant_id = Uuid::new_v4();

        let mut transaction = tenant_pool
            .begin(tenant_id)
            .await
            .expect("开启租户事务失败");
        let inside: Option<String> =
            sqlx::query_scalar("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(&mut **transaction.transaction_mut())
                .await
                .expect("读取租户上下文失败");
        assert_eq!(inside, Some(tenant_id.to_string()));

        transaction.commit().await.expect("提交租户事务失败");

        let after: Option<String> =
            sqlx::query_scalar("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(&tenant_pool.pool)
                .await
                .expect("读取租户上下文失败");
        // 事务结束后 GUC 被清除；PostgreSQL 对已存在但未设置的 GUC 返回空字符串。
        assert_eq!(after.as_deref(), Some(""));
    }
}
