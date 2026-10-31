use sqlx::migrate::MigrateError;

/// 数据库基础设施错误。
///
/// 业务错误不放在这里；本类型只覆盖配置、连接、迁移与租户上下文。
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    /// 缺少必需的环境变量。
    #[error("缺少环境变量 {name}")]
    MissingEnv {
        /// 环境变量名。
        name: &'static str,
    },

    /// 连接串无法解析。
    #[error("数据库配置无效：{reason}")]
    InvalidConfig {
        /// 面向人的原因说明，不包含连接串本身。
        reason: &'static str,
    },

    /// 迁移执行失败。
    #[error("数据库迁移失败")]
    Migrate {
        /// 底层迁移错误。
        #[source]
        source: MigrateError,
    },

    /// 开启事务或设置租户上下文失败。
    #[error("设置租户上下文失败")]
    TenantContext {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },

    /// 连接池健康检查失败。
    #[error("{pool} 连接池健康检查失败")]
    Health {
        /// 连接池名称，用于定位租户池或管理池。
        pool: &'static str,
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },

    /// 提交事务失败。
    #[error("提交事务失败")]
    Commit {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
