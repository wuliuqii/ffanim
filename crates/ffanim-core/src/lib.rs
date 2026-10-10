//! ffanim-core：对象模型 + 时间轴 + 动画系统。
//!
//! 本 crate 不依赖任何渲染实现（kurbo 仅为几何类型），是整个 workspace 的依赖根。

pub mod error;
pub mod item;
pub mod style;
pub mod text;

pub use error::{Error, Result};
pub use item::{DrawList, Item, ItemId, Paint, Shape};
pub use style::{Color, Style, Theme};
pub use text::Font;

/// 几何类型重导出（kurbo 全 workspace 锁同版本，路径类型跨 crate 天然一致）。
pub use kurbo;
