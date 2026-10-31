use sqlx::postgres::PgConnectOptions;

use crate::DbError;

/// 双连接池配置。
///
/// 租户池连接受 RLS 约束的角色，管理池连接可跨租户读取例外表的角色；
/// 两者的连接串不得互换。
#[derive(Clone)]
pub struct DbConfig {
    tenant: PgConnectOptions,
    admin: PgConnectOptions,
    tenant_max_connections: u32,
    admin_max_connections: u32,
}

impl DbConfig {
    /// 租户池默认连接上限，对应 api 租户池。
    pub const DEFAULT_TENANT_MAX_CONNECTIONS: u32 = 10;

    /// 管理池默认连接上限，对应 api 与 worker 的管理池。
    pub const DEFAULT_ADMIN_MAX_CONNECTIONS: u32 = 5;

    /// 从两个连接串构造配置。
    ///
    /// # Errors
    /// 任一连接串无法解析时返回 [`DbError::InvalidConfig`]。
    pub fn from_urls(tenant_url: &str, admin_url: &str) -> Result<Self, DbError> {
        let tenant =
            tenant_url
                .parse::<PgConnectOptions>()
                .map_err(|_| DbError::InvalidConfig {
                    reason: "租户连接串格式无效",
                })?;
        let admin = admin_url
            .parse::<PgConnectOptions>()
            .map_err(|_| DbError::InvalidConfig {
                reason: "管理连接串格式无效",
            })?;
        Ok(Self::from_options(tenant, admin))
    }

    /// 从已解析的连接选项构造配置，使用默认连接上限。
    #[must_use]
    pub fn from_options(tenant: PgConnectOptions, admin: PgConnectOptions) -> Self {
        Self {
            tenant,
            admin,
            tenant_max_connections: Self::DEFAULT_TENANT_MAX_CONNECTIONS,
            admin_max_connections: Self::DEFAULT_ADMIN_MAX_CONNECTIONS,
        }
    }

    /// 从环境变量读取双连接池配置。
    ///
    /// # Errors
    /// 缺少 `DATABASE_URL_TENANT` 或 `DATABASE_URL_ADMIN`，或连接串非法时返回错误。
    pub fn from_env() -> Result<Self, DbError> {
        let tenant_url = std::env::var("DATABASE_URL_TENANT").map_err(|_| DbError::MissingEnv {
            name: "DATABASE_URL_TENANT",
        })?;
        let admin_url = std::env::var("DATABASE_URL_ADMIN").map_err(|_| DbError::MissingEnv {
            name: "DATABASE_URL_ADMIN",
        })?;
        Self::from_urls(&tenant_url, &admin_url)
    }

    pub(crate) fn tenant_options(&self) -> &PgConnectOptions {
        &self.tenant
    }

    pub(crate) fn admin_options(&self) -> &PgConnectOptions {
        &self.admin
    }

    pub(crate) fn tenant_max_connections(&self) -> u32 {
        self.tenant_max_connections
    }

    pub(crate) fn admin_max_connections(&self) -> u32 {
        self.admin_max_connections
    }
}
