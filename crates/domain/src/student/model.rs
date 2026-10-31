//! 学员模型与错误。

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 学员；只保存昵称或姓名、年级与科目。
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Student {
    /// 学员主键。
    pub id: Uuid,
    /// 姓名或昵称。
    pub name: String,
    /// 年级。
    pub grade: Option<String>,
    /// 科目。
    pub subject: Option<String>,
    /// 创建时间。
    pub created_at: DateTime<Utc>,
}

/// 创建或更新学员的输入字段。
#[derive(Debug, Clone)]
pub struct StudentInput<'a> {
    /// 姓名或昵称。
    pub name: &'a str,
    /// 年级。
    pub grade: Option<&'a str>,
    /// 科目。
    pub subject: Option<&'a str>,
}

/// 删除结果。
#[derive(Debug, Clone, Serialize)]
pub struct DeletedStudent {
    /// 被删除的学员。
    pub id: Uuid,
    /// 级联删除的反馈数量。
    pub feedback_count: i64,
}

/// 学员操作错误。
#[derive(Debug, thiserror::Error)]
pub enum StudentError {
    /// 学员不存在或不属于当前租户。
    #[error("学员不存在")]
    NotFound,
    /// 姓名为空或过长。
    #[error("学员姓名无效")]
    InvalidName,
    /// 删除确认姓名不匹配。
    #[error("学员姓名不匹配")]
    NameMismatch,
    /// 开启或提交租户事务失败。
    #[error("学员事务失败")]
    Transaction {
        /// 底层数据库错误。
        #[source]
        source: db::DbError,
    },
    /// 查询或写入失败。
    #[error("学员数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
