//! 课次模型、分页游标与错误。

use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use uuid::Uuid;

/// 课次。
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Lesson {
    /// 课次主键。
    pub id: Uuid,
    /// 所属班级。
    pub class_id: Uuid,
    /// 上课日期。
    pub lesson_date: NaiveDate,
    /// 课次主题。
    pub topic: Option<String>,
    /// 状态，仅允许 `draft` 与 `done`。
    pub status: String,
    /// 创建时间。
    pub created_at: DateTime<Utc>,
}

/// 创建课次的输入字段。
#[derive(Debug, Clone)]
pub struct LessonInput<'a> {
    /// 所属班级。
    pub class_id: Uuid,
    /// 上课日期。
    pub lesson_date: NaiveDate,
    /// 课次主题。
    pub topic: Option<&'a str>,
}

/// 列表分页游标：上一页最后一行的排序键。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LessonCursor {
    /// 上课日期。
    pub lesson_date: NaiveDate,
    /// 课次主键，用于日期相同时稳定排序。
    pub id: Uuid,
}

/// 课次列表筛选条件。
#[derive(Debug, Clone, Copy)]
pub struct LessonFilter {
    /// 起始日期（含）。
    pub from: Option<NaiveDate>,
    /// 结束日期（含）。
    pub to: Option<NaiveDate>,
    /// 每页数量。
    pub limit: i64,
    /// 上一页返回的游标。
    pub cursor: Option<LessonCursor>,
}

/// 课次分页结果。
#[derive(Debug)]
pub struct LessonPage {
    /// 当前页数据。
    pub items: Vec<Lesson>,
    /// 下一页游标；没有更多数据时为 `None`。
    pub next_cursor: Option<LessonCursor>,
}

/// 课次操作错误。
#[derive(Debug, thiserror::Error)]
pub enum LessonError {
    /// 班级不存在或不属于当前租户。
    #[error("班级不存在")]
    ClassNotFound,
    /// 主题过长。
    #[error("课次主题无效")]
    InvalidTopic,
    /// 分页数量超出范围。
    #[error("分页数量无效")]
    InvalidLimit,
    /// 起始日期晚于结束日期。
    #[error("日期范围无效")]
    InvalidDateRange,
    /// 开启或提交租户事务失败。
    #[error("课次事务失败")]
    Transaction {
        /// 底层数据库错误。
        #[source]
        source: db::DbError,
    },
    /// 查询或写入失败。
    #[error("课次数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
