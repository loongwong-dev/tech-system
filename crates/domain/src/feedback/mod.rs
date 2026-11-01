//! 反馈领域：教师原文读写、模板润色与批量生成。
//!
//! `draft` 负责草稿读写与状态校验，`service` 负责生成。生成结果只写
//! `rendered`，教师原文 `notes` 由 `draft` 独占写入，两条路径互不覆盖。

mod draft;
mod model;
mod repository;
mod service;
mod tags;
mod template;

pub use draft::{
    DraftError, DraftInput, FeedbackDraft, MAX_JSON_BYTES as DRAFT_MAX_JSON_BYTES,
    MAX_NOTES_CHARS as DRAFT_MAX_NOTES_CHARS, MAX_TAG_CHARS as DRAFT_MAX_TAG_CHARS,
    MAX_TAGS as DRAFT_MAX_TAGS, list_lesson_drafts, save_draft,
};
pub use model::{Feedback, GeneratedFeedback, GenerationError};
pub use service::{generate_batch, generate_single, lesson_feedback_count};
pub use tags::{
    CatalogDimension, CatalogTag, InheritedTags, TagQueryError, TagTone, latest_student_tags,
    list_catalog,
};
