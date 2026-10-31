//! 反馈生成模型与错误。
//!
//! 生成结果只写 `rendered`，教师原文 `notes` 永远保留。

use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

/// 参与生成的反馈字段。
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Feedback {
    /// 反馈主键。
    pub id: Uuid,
    /// 所属课次。
    pub lesson_id: Uuid,
    /// 教师原文，生成不得覆盖。
    pub notes: String,
    /// 知识点标签。
    pub knowledge_tags: Vec<String>,
    /// 上一次生成结果。
    pub rendered: Option<Value>,
    /// 反馈状态。
    pub status: String,
    /// 已重新生成次数。
    pub regen_count: i16,
}

/// 单份生成结果。
#[derive(Debug, Clone, Serialize)]
pub struct GeneratedFeedback {
    /// 反馈主键。
    pub feedback_id: Uuid,
    /// 生成正文。
    pub text: String,
    /// 生成方式：当前为 `template`。
    pub generated_by: String,
    /// 生成规则版本。
    pub prompt_version: String,
    /// 是否命中幂等键，直接复用上次结果。
    pub reused: bool,
}

/// 反馈生成错误。
#[derive(Debug, thiserror::Error)]
pub enum GenerationError {
    /// 反馈不存在或不属于当前租户。
    #[error("反馈不存在")]
    NotFound,
    /// 课次不存在或不属于当前租户。
    #[error("课次不存在")]
    LessonNotFound,
    /// 课次下没有可生成的反馈。
    #[error("课次没有反馈")]
    LessonEmpty,
    /// 当前状态不允许生成，例如已分享。
    #[error("当前状态不允许生成")]
    InvalidState,
    /// 重新生成次数达到上限。
    #[error("重新生成次数已达上限")]
    RegenerationLimit,
    /// 开启或提交租户事务失败。
    #[error("反馈生成事务失败")]
    Transaction {
        /// 底层数据库错误。
        #[source]
        source: db::DbError,
    },
    /// 查询或写入失败。
    #[error("反馈生成数据库操作失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}
