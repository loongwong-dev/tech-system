//! 学员、班级、课次、反馈与分享的领域模型和业务规则。
//!
//! 允许依赖 `db` 与 `shared`；不得依赖 `platform` 或 `api`。
//!
//! 反馈模块当前实现两条路径：教师原文与标签的草稿读写（`draft`），以及
//! 不依赖外部模型的模板生成。模板生成只写 `rendered`，不覆盖 `notes`。

mod class;
mod feedback;
mod lesson;
mod student;

pub use class::{
    Class, ClassError, ClassInput, archive_class, create_class, delete_class, list_classes,
    update_class,
};
pub use feedback::{
    CatalogDimension, CatalogTag, DRAFT_MAX_JSON_BYTES, DRAFT_MAX_NOTES_CHARS, DRAFT_MAX_TAG_CHARS,
    DRAFT_MAX_TAGS, DraftError, DraftInput, Feedback, FeedbackDraft, GeneratedFeedback,
    GenerationError, InheritedTags, TagQueryError, TagTone, generate_batch, generate_single,
    latest_student_tags, lesson_feedback_count, list_catalog, list_lesson_drafts, save_draft,
};
pub use lesson::{
    DEFAULT_LIMIT as LESSON_DEFAULT_LIMIT, Lesson, LessonCursor, LessonError, LessonFilter,
    LessonInput, LessonPage, MAX_LIMIT as LESSON_MAX_LIMIT,
    MAX_TOPIC_CHARS as LESSON_MAX_TOPIC_CHARS, create_lesson, list_lessons,
};
pub use student::{
    DeletedStudent, Student, StudentError, StudentInput, archive_student, create_student,
    delete_student, list_students, update_student,
};
