//! ffanim-core：对象模型 + 时间轴 + 动画系统。
//!
//! 本 crate 不依赖任何渲染/编码实现，是整个 workspace 的依赖根。

pub mod error;
pub mod style;

pub use error::{Error, Result};
pub use style::{Color, Style, Theme};
