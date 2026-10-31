//! 学员领域：最小字段、归档、删除确认与级联。

mod model;
mod repository;
mod service;

pub use model::{DeletedStudent, Student, StudentError, StudentInput};
pub use service::{archive_student, create_student, delete_student, list_students, update_student};
