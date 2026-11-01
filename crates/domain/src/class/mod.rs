//! 班级领域：维护班级，删除时保护已有课次。

mod model;
mod repository;
mod service;

pub use model::{Class, ClassError, ClassInput};
pub use service::{archive_class, create_class, delete_class, list_classes, update_class};
