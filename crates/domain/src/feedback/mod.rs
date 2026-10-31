//! 反馈生成领域：读取草稿、模板润色与批量处理。

mod model;
mod repository;
mod service;
mod template;

pub use model::{Feedback, GeneratedFeedback, GenerationError};
pub use service::{generate_batch, generate_single, lesson_feedback_count};
