//! 求值期信号：值跟踪器与 updater（值→对象 映射函数）。
//!
//! tracker 是隐形信号源（不产生绘制），updater 把 tracker 当前值重建为
//! 对象形状——无可变状态（Send + Sync），与两阶段模型的并行求值兼容。

use crate::{Item, ItemId};
use std::sync::Arc;

/// 值跟踪器句柄（[`crate::timeline::Timeline::add_tracker`] 返回，隐形对象，仅供 updater 依赖）。
pub type TrackerId = usize;

/// 值→对象 映射函数类型（updater 信号核心）。
pub type ValueFn = Arc<dyn Fn(&[f64]) -> Item + Send + Sync>;

/// 求值期信号：把若干 tracker 值映射为一个对象（构建期注册的纯函数）。
///
/// 渲染期每帧以依赖 tracker 的当前值调用 [`Updater::eval`] 重建形状，
/// 无可变状态（Send + Sync），与两阶段模型的并行求值兼容。
pub struct Updater {
    pub item_id: ItemId,
    pub deps: Vec<TrackerId>,
    f: ValueFn,
}

impl Updater {
    pub fn new(
        item_id: ItemId,
        deps: Vec<TrackerId>,
        f: impl Fn(&[f64]) -> Item + Send + Sync + 'static,
    ) -> Self {
        Self {
            item_id,
            deps,
            f: Arc::new(f),
        }
    }

    /// 以当前帧 tracker 值重建对象。
    pub fn eval(&self, values: &[f64]) -> Item {
        (self.f)(values)
    }
}

impl Clone for Updater {
    fn clone(&self) -> Self {
        Self {
            item_id: self.item_id,
            deps: self.deps.clone(),
            f: Arc::clone(&self.f),
        }
    }
}

impl std::fmt::Debug for Updater {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Updater")
            .field("item_id", &self.item_id)
            .field("deps", &self.deps)
            .finish_non_exhaustive()
    }
}
