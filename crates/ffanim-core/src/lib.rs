//! ffanim-core：对象模型 + 时间轴 + 动画系统。
//!
//! 本 crate 不依赖任何渲染实现（kurbo 仅为几何类型），是整个 workspace 的依赖根。

pub mod anim;
pub mod camera;
pub mod coords;
pub mod error;
pub mod eval;
pub mod hit_test;
pub mod item;
pub mod layout;
pub mod match_shape;
pub mod math;
pub mod ops;
pub mod params;
pub mod path_util;
pub mod rate;
pub mod space3;
pub mod style;
pub mod svg;
pub mod table;
pub mod text;
pub mod timeline;
pub mod tracker;

pub use anim::{Anim, AnimKind, PoseDelta};
pub use camera::Camera;
pub use coords::Axes;
pub use error::{Error, Result};
pub use eval::{CompiledTimeline, render_frame};
pub use item::{DrawList, ImageAsset, Item, ItemId, Paint, Shape};
pub use layout::{Direction, arrange};
pub use match_shape::{MatchPlan, match_shapes};
pub use ops::BoolOp;
pub use params::{load_params, params_from_str};
pub use path_util::{lerp_affine, lerp_path, partial_path};
pub use rate::{RateFunc, RateTable};
pub use style::{Color, Style, Theme};
pub use svg::{from_svg_file, from_svg_str};
pub use table::{TableOptions, TableSpec, table};
pub use text::{Font, LATIN_FAMILIES, find_system_font, outline_glyph};
pub use timeline::{ScheduledAnim, Timeline};
pub use tracker::{TrackerId, Updater};

/// 几何类型重导出（kurbo 全 workspace 锁同版本，路径类型跨 crate 天然一致）。
pub use kurbo;
