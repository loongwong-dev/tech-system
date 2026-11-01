//! 反馈草稿模型与错误。

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

/// 反馈草稿：教师可编辑字段与当前状态。
///
/// 一份草稿由 `(lesson_id, student_id)` 唯一确定，一节课一名学员只有一份反馈。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FeedbackDraft {
    /// 反馈主键，创建时由客户端生成。
    pub id: Uuid,
    /// 所属课次。
    pub lesson_id: Uuid,
    /// 所属学员。
    pub student_id: Uuid,
    /// 教师原文，任何生成流程都不得覆盖。
    pub notes: String,
    /// 打分对象，结构由前端定义。
    pub ratings: Value,
    /// 知识点标签。
    pub knowledge_tags: Vec<String>,
    /// 作业对象，结构由前端定义。
    pub homework: Value,
    /// 反馈状态，取值与数据库 CHECK 约束一致。
    pub status: String,
    /// 最近一次写入时间。
    pub updated_at: DateTime<Utc>,
}

/// 写入草稿的输入。
///
/// `notes`、`ratings`、`knowledge_tags`、`homework` 为整体替换语义：
/// 防抖提交必须带上完整状态，缺字段会被拒绝而不是清空已有内容。
#[derive(Debug, Clone)]
pub struct DraftInput {
    /// 所属课次。
    pub lesson_id: Uuid,
    /// 所属学员。
    pub student_id: Uuid,
    /// 教师原文。
    pub notes: String,
    /// 打分对象。
    pub ratings: Value,
    /// 知识点标签。
    pub knowledge_tags: Vec<String>,
    /// 作业对象。
    pub homework: Value,
}

/// 草稿身份，用于判断路径 id 是否指向同一份草稿。
#[derive(Debug, Clone, sqlx::FromRow)]
pub(crate) struct DraftIdentity {
    /// 所属课次。
    pub lesson_id: Uuid,
    /// 所属学员。
    pub student_id: Uuid,
    /// 当前状态。
    pub status: String,
}

/// 反馈草稿读写错误。
#[derive(Debug, thiserror::Error)]
pub enum DraftError {
    /// 课次不存在或不属于当前租户。
    #[error("课次不存在")]
    LessonNotFound,
    /// 学员不存在或不属于当前租户。
    #[error("学员不存在")]
    StudentNotFound,
    /// 反馈不存在或不属于当前租户。
    #[error("反馈不存在")]
    NotFound,
    /// 当前状态不允许编辑，只有 `draft` 可写。
    #[error("当前状态不允许编辑")]
    InvalidTransition,
    /// 路径 id 已被同一租户内的其他反馈占用。
    #[error("反馈 ID 已被占用")]
    IdReused,
    /// 教师原文超长。
    #[error("教师原文无效")]
    InvalidNotes,
    /// 标签数量或单个标签长度非法。
    #[error("知识点标签无效")]
    InvalidTags,
    /// `ratings` 不是对象或超出体积上限。
    #[error("打分对象无效")]
    InvalidRatings,
    /// `homework` 不是对象或超出体积上限。
    #[error("作业对象无效")]
    InvalidHomework,
    /// 路径 id 与已提交的记录冲突，且在当前租户内不可见（通常属于其他租户）。
    ///
    /// 由 `sqlx` 的主键冲突触发，重试后仍冲突才返回给调用方，对外映射为 404。
    #[error("反馈 ID 冲突")]
    IdConflict {
        /// 底层数据库错误，保留约束名与细节用于排查。
        #[source]
        source: sqlx::Error,
    },
    /// 开启或提交租户事务失败。
    #[error("反馈草稿事务失败")]
    Transaction {
        /// 底层数据库错误。
        #[source]
        source: db::DbError,
    },
    /// 查询或写入失败。
    #[error("反馈草稿数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
