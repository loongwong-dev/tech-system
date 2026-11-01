//! 反馈草稿领域：按课次批量读取与教师原文写入。
//!
//! 草稿身份由 `(lesson_id, student_id)` 唯一确定，一节课一名学员只有一份反馈。
//! 所有读写都在租户事务内完成，跨租户的资源一律视为不存在。

mod model;
mod repository;
mod service;

pub use model::{DraftError, DraftInput, FeedbackDraft};
pub use service::{
    MAX_JSON_BYTES, MAX_NOTES_CHARS, MAX_TAG_CHARS, MAX_TAGS, list_lesson_drafts, save_draft,
};
