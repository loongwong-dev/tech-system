//! axum 应用装配、路由与中间件。
//!
//! 允许依赖 `db`、`jobs`、`platform`、`ai`、`render` 与 `shared`；当前只装配
//! 基础路由、统一错误模型与健康检查，业务端点随后续任务加入。

mod app;
mod error;
mod routes;
mod state;

pub use app::build_router;
pub use error::{ApiError, FieldError};
pub use state::AppState;
