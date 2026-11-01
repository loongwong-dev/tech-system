//! 反馈标签目录：维度、语气权重与推导。
//!
//! 目录是产品资产而非配置：五个维度与 17 个标签硬编码在本模块，前端通过
//! 目录接口渲染选择器，服务端据此校验标签并推导 `tone`。
//!
//! 不负责话术映射与模板生成（见 #35），也不提供标签管理界面。

mod model;
mod repository;
mod service;

pub use model::{TagInputError, TagTone, validate};
pub use service::{
    CatalogDimension, CatalogTag, InheritedTags, TagQueryError, latest_student_tags, list_catalog,
};
