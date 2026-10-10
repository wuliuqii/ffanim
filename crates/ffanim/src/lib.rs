//! ffanim：门面 crate，re-export 核心类型与各子系统，构成用户面向的 API。
//!
//! 用户只依赖 `ffanim`，不直接依赖内部 crate。

pub use ffanim_core::{Color, Error, Result, Style, Theme};

/// 项目版本（供 CLI 与元信息使用）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
