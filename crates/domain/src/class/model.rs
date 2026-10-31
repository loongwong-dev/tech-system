//! 班级模型与错误。

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 班级。
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Class {
    /// 班级主键。
    pub id: Uuid,
    /// 班级名称。
    pub name: String,
    /// 科目。
    pub subject: Option<String>,
    /// 创建时间。
    pub created_at: DateTime<Utc>,
}

/// 创建或更新班级的输入字段。
#[derive(Debug, Clone)]
pub struct ClassInput<'a> {
    /// 班级名称。
    pub name: &'a str,
    /// 科目。
    pub subject: Option<&'a str>,
}

/// 班级操作错误。
#[derive(Debug, thiserror::Error)]
pub enum ClassError {
    /// 班级不存在或不属于当前租户。
    #[error("班级不存在")]
    NotFound,
    /// 名称为空或过长。
    #[error("班级名称无效")]
    InvalidName,
    /// 班级下仍有课次，拒绝删除。
    #[error("班级仍有课次")]
    HasLessons,
    /// 开启或提交租户事务失败。
    #[error("班级事务失败")]
    Transaction {
        /// 底层数据库错误。
        #[source]
        source: db::DbError,
    },
    /// 查询或写入失败。
    #[error("班级数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
