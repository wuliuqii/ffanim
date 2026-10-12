//! ffanim-core：对象模型 + 时间轴 + 动画系统。
//!
//! 本 crate 不依赖任何渲染实现（kurbo 仅为几何类型），是整个 workspace 的依赖根。

pub mod coords;
pub mod error;
pub mod item;
pub mod path_util;
pub mod rate;
pub mod style;
pub mod text;
pub mod timeline;

pub use coords::Axes;
pub use error::{Error, Result};
pub use item::{DrawList, Item, ItemId, Paint, Shape};
pub use path_util::{lerp_affine, lerp_path, partial_path};
pub use rate::RateFunc;
pub use style::{Color, Style, Theme};
pub use text::{Font, outline_glyph};
pub use timeline::{Anim, AnimKind, CompiledTimeline, ScheduledAnim, Timeline, render_frame};

/// 几何类型重导出（kurbo 全 workspace 锁同版本，路径类型跨 crate 天然一致）。
pub use kurbo;
