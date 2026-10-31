//! 学员、班级、课次、反馈与分享的领域模型和业务规则。
//!
//! 允许依赖 `db` 与 `shared`；不得依赖 `platform` 或 `api`。
//!
//! 当前只实现反馈模板生成：它是不依赖外部模型的兜底路径，也是接入 AI
//! 适配器前的可运行闭环。模板生成不得覆盖教师输入的 `notes`。

mod class;
mod feedback;
mod student;

pub use class::{
    Class, ClassError, ClassInput, create_class, delete_class, list_classes, update_class,
};
pub use feedback::{
    Feedback, GeneratedFeedback, GenerationError, generate_batch, generate_single,
    lesson_feedback_count,
};
pub use student::{
    DeletedStudent, Student, StudentError, StudentInput, archive_student, create_student,
    delete_student, list_students, update_student,
};
