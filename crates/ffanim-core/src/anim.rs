//! 动画词汇表：动画类型（[`AnimKind`]）与构建期描述（[`Anim`]）。
//!
//! 对应 manim 的 `Create/FadeIn/FadeOut/Transform` 与 `rate_func`；
//! 只描述"做什么"，调度（何时做）属于 [`crate::timeline::Timeline`]。

use crate::rate::RateFunc;
use crate::{ItemId, tracker::TrackerId};
use kurbo::Vec2;

/// 动画类型。
#[derive(Debug, Clone)]
pub enum AnimKind {
    /// 从无到有：沿路径逐段画出（描边生长），填充随之淡入。
    Create(ItemId),
    /// 淡入：透明度 0→1，可选从偏移处平移进入、从 90% 缩放放大。
    FadeIn { id: ItemId, shift: Vec2 },
    /// 淡出：透明度 1→0，结束后对象隐藏。
    FadeOut(ItemId),
    /// 形变：`from` 形变为 `to`（几何/颜色/变换插值；元素序列不一致时退化为交叉淡化）。
    Transform { from: ItemId, to: ItemId },
    /// 值跟踪器动画：`from` → `to`（updater 求值期信号的数据来源，本身不产生绘制）。
    SetValue {
        tracker: TrackerId,
        from: f64,
        to: f64,
    },
}

impl AnimKind {
    /// 人类可读摘要（`timeline` 子命令与 agent 检查用）。
    pub fn describe(&self) -> String {
        match self {
            AnimKind::Create(id) => format!("Create(item#{id})"),
            AnimKind::FadeIn { id, shift } => {
                if *shift == Vec2::ZERO {
                    format!("FadeIn(item#{id})")
                } else {
                    format!("FadeIn(item#{id}, shift=({:.0},{:.0}))", shift.x, shift.y)
                }
            }
            AnimKind::FadeOut(id) => format!("FadeOut(item#{id})"),
            AnimKind::Transform { from, to } => format!("Transform(item#{from}→item#{to})"),
            AnimKind::SetValue { tracker, from, to } => {
                format!("SetValue(tracker#{tracker}: {from:.3}→{to:.3})")
            }
        }
    }
}

/// 动画描述（构建期）。
#[derive(Debug, Clone)]
pub struct Anim {
    pub kind: AnimKind,
    pub duration_s: f64,
    pub rate: RateFunc,
}

impl Anim {
    pub fn create(id: ItemId) -> Self {
        Self::of(AnimKind::Create(id))
    }

    pub fn fade_in(id: ItemId) -> Self {
        Self::of(AnimKind::FadeIn {
            id,
            shift: Vec2::ZERO,
        })
    }

    pub fn fade_out(id: ItemId) -> Self {
        Self::of(AnimKind::FadeOut(id))
    }

    pub fn transform(from: ItemId, to: ItemId) -> Self {
        Self::of(AnimKind::Transform { from, to })
    }

    /// 值跟踪器动画（from 取构建期该 tracker 的当前值）。
    pub fn set_value(tracker: TrackerId, to: f64) -> Self {
        Self::of(AnimKind::SetValue {
            tracker,
            from: f64::NAN, // 由 Timeline::play 填充（构建期当前值）
            to,
        })
    }

    fn of(kind: AnimKind) -> Self {
        Self {
            kind,
            duration_s: 1.0,
            rate: RateFunc::Smooth,
        }
    }

    /// 时长（秒）。
    pub fn during(mut self, seconds: f64) -> Self {
        assert!(seconds > 0.0, "动画时长必须为正");
        self.duration_s = seconds;
        self
    }

    /// 缓动函数。
    pub fn rate(mut self, rate: RateFunc) -> Self {
        self.rate = rate;
        self
    }

    /// FadeIn 的进入偏移。
    pub fn shift(mut self, dx: f64, dy: f64) -> Self {
        if let AnimKind::FadeIn { shift, .. } = &mut self.kind {
            *shift = Vec2::new(dx, dy);
        }
        self
    }
}
