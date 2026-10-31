//! 跨模块共享的错误、配置、ID 与通用校验。
//!
//! 本 crate 是依赖树的叶子：不得依赖任何其他 workspace crate。

/// 当前 workspace 的稳定标识，供日志与配置默认值使用。
pub const WORKSPACE_NAME: &str = "tech-system";
