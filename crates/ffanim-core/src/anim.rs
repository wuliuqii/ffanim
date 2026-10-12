//! 动画词汇表：动画类型（[`AnimKind`]）与构建期描述（[`Anim`]）。
//!
//! 对应 manim 的 `Create/FadeIn/FadeOut/Transform` 与 `rate_func`；
//! 只描述"做什么"，调度（何时做）属于 [`crate::timeline::Timeline`]。

use crate::layout::Direction;
use crate::rate::RateFunc;
use crate::{ItemId, tracker::TrackerId};
use kurbo::{Affine, Point, Vec2};

/// 动画类型。
#[derive(Debug, Clone)]
pub enum AnimKind {
    /// 从无到有：沿路径逐段画出（描边生长），填充随之淡入。
    Create(ItemId),
    /// 描边收尾：沿路径逐段收回（create 的镜像），填充随之淡出，结束后对象隐藏。
    Uncreate(ItemId),
    /// 淡入：透明度 0→1，可选从偏移处平移进入、从 `scale_from` 缩放长出、
    /// 旋转 `spin`（弧度余量）进入。
    FadeIn {
        /// 目标对象。
        id: ItemId,
        /// 进入时的平移偏移（零向量 = 原地淡入）。
        shift: Vec2,
        /// 进入起始缩放（相对完整尺寸；默认 0.9 = 轻微放大进入）。
        scale_from: f64,
        /// 进入旋转余量（弧度，alpha=0 时偏移量；默认 0）。
        spin: f64,
    },
    /// 淡出：透明度 1→0，结束后对象隐藏。
    FadeOut(ItemId),
    /// 形变：`from` 形变为 `to`（几何/颜色/变换插值；元素序列不一致时退化为交叉淡化）。
    Transform {
        /// 形变起点对象。
        from: ItemId,
        /// 形变终点对象（完成后保留在画布上）。
        to: ItemId,
    },
    /// 值跟踪器动画：`from` → `to`（updater 求值期信号的数据来源，本身不产生绘制）。
    SetValue {
        /// 目标跟踪器。
        tracker: TrackerId,
        /// 起始值（由 Timeline 填充为构建期当前值）。
        from: f64,
        /// 结束值。
        to: f64,
    },
    /// 变换到目标：对象 transform 插值到 `to`（段前取构建期基线，段后停在目标；
    /// 相机推拉摇移与目标位姿的通用词汇）。
    TransformTo {
        /// 目标对象。
        id: ItemId,
        /// 起始变换（由 Timeline 填充为构建期当前值）。
        from: Affine,
        /// 目标变换（完成后保留为对象基线）。
        to: Affine,
    },
    /// 相对位姿动作（构建期中间表示，manim `.animate.*` 对应词汇）：
    /// [`Timeline::play`](crate::Timeline::play) 时按构建期基线解析为绝对
    /// [`AnimKind::TransformTo`]，编译表内不出现此变体。
    Pose {
        /// 目标对象。
        id: ItemId,
        /// 相对位姿变化。
        delta: PoseDelta,
    },
}

/// 相对位姿目标：`resolve` 由基线变换与包围盒中心算出绝对目标变换。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoseDelta {
    /// 世界系平移 delta。
    Shift(Vec2),
    /// 平移使对象 bbox 中心落在目标点。
    MoveTo(Point),
    /// 绕 bbox 中心旋转（弧度；正 = **视觉逆时针**，manim 语义）。
    Rotate(f64),
    /// 绕 bbox 中心缩放系数。
    Scale(f64),
}

impl PoseDelta {
    /// 由基线变换 `from` 与其包围盒中心 `center` 计算绝对目标变换。
    pub fn resolve(self, from: Affine, center: Point) -> Affine {
        let c = Vec2::new(center.x, center.y);
        match self {
            PoseDelta::Shift(d) => Affine::translate(d) * from,
            PoseDelta::MoveTo(p) => {
                Affine::translate(Vec2::new(p.x - center.x, p.y - center.y)) * from
            }
            PoseDelta::Rotate(a) => {
                // 视觉逆时针 = 数学顺时针（y 向下）→ 角度取负
                Affine::translate(c) * Affine::rotate(-a) * Affine::translate(-c) * from
            }
            PoseDelta::Scale(k) => {
                Affine::translate(c) * Affine::scale(k) * Affine::translate(-c) * from
            }
        }
    }
}

impl AnimKind {
    /// 人类可读摘要（`timeline` 子命令与 agent 检查用）。
    pub fn describe(&self) -> String {
        match self {
            AnimKind::Create(id) => format!("Create(item#{id})"),
            AnimKind::Uncreate(id) => format!("Uncreate(item#{id})"),
            AnimKind::FadeIn {
                id,
                shift,
                scale_from,
                spin,
            } => {
                let mut s = format!("FadeIn(item#{id}");
                if *shift != Vec2::ZERO {
                    s += &format!(", shift=({:.0},{:.0})", shift.x, shift.y);
                }
                if (*scale_from - 0.9).abs() > 1e-9 {
                    s += &format!(", scale_from={scale_from:.2}");
                }
                if spin.abs() > 1e-9 {
                    s += &format!(", spin={spin:.2}rad");
                }
                s + ")"
            }
            AnimKind::FadeOut(id) => format!("FadeOut(item#{id})"),
            AnimKind::Transform { from, to } => format!("Transform(item#{from}→item#{to})"),
            AnimKind::SetValue { tracker, from, to } => {
                format!("SetValue(tracker#{tracker}: {from:.3}→{to:.3})")
            }
            AnimKind::TransformTo { id, .. } => format!("TransformTo(item#{id})"),
            AnimKind::Pose { id, delta } => match delta {
                PoseDelta::Shift(d) => format!("Pose(item#{id}, shift=({:.0},{:.0}))", d.x, d.y),
                PoseDelta::MoveTo(p) => format!("Pose(item#{id}, move_to=({:.0},{:.0}))", p.x, p.y),
                PoseDelta::Rotate(a) => format!("Pose(item#{id}, rotate={a:.2}rad)"),
                PoseDelta::Scale(k) => format!("Pose(item#{id}, scale={k:.2})"),
            },
        }
    }
}

/// 动画描述（构建期）。
#[derive(Debug, Clone)]
pub struct Anim {
    /// 动画类型（做什么）。
    pub kind: AnimKind,
    /// 时长（秒），默认 1.0，经 [`Anim::during`] 修改。
    pub duration_s: f64,
    /// 缓动函数（默认 [`RateFunc::Linear`]）。
    pub rate: RateFunc,
}

impl Anim {
    /// 创建 Create 动画：沿路径逐段画出。
    pub fn create(id: ItemId) -> Self {
        Self::of(AnimKind::Create(id))
    }

    /// 创建 FadeIn 动画：原地淡入（轻微放大进入）。
    pub fn fade_in(id: ItemId) -> Self {
        Self::of(AnimKind::FadeIn {
            id,
            shift: Vec2::ZERO,
            scale_from: 0.9,
            spin: 0.0,
        })
    }

    /// 创建 Uncreate 动画：描边逐段收回（create 反放），结束后对象隐藏。
    ///
    /// 非路径形状（图片/组合内无路径）退化为纯淡出。
    pub fn uncreate(id: ItemId) -> Self {
        Self::of(AnimKind::Uncreate(id))
    }

    /// 创建 FadeIn 方向家族（manim `FadeIn(point=dir)` 对应）：从 `dir` 侧
    /// 距离 `dist` 处滑入淡入。`Direction::Left` = 起始在对象左侧。
    pub fn fade_in_from(id: ItemId, dir: Direction, dist: f64) -> Self {
        let shift = match dir {
            Direction::Up => Vec2::new(0.0, -dist),
            Direction::Down => Vec2::new(0.0, dist),
            Direction::Left => Vec2::new(-dist, 0.0),
            Direction::Right => Vec2::new(dist, 0.0),
        };
        Self::of(AnimKind::FadeIn {
            id,
            shift,
            scale_from: 0.9,
            spin: 0.0,
        })
    }

    /// 创建 GrowFromCenter 动画（manim 对应）：从包围盒中心 0 缩放长出淡入。
    pub fn grow_from_center(id: ItemId) -> Self {
        Self::of(AnimKind::FadeIn {
            id,
            shift: Vec2::ZERO,
            scale_from: 0.0,
            spin: 0.0,
        })
    }

    /// 创建 GrowFromEdge 动画（manim 对应）：从 `dir` 侧 `dist` 处 0 缩放长出淡入。
    pub fn grow_from_edge(id: ItemId, dir: Direction, dist: f64) -> Self {
        let shift = match dir {
            Direction::Up => Vec2::new(0.0, -dist),
            Direction::Down => Vec2::new(0.0, dist),
            Direction::Left => Vec2::new(-dist, 0.0),
            Direction::Right => Vec2::new(dist, 0.0),
        };
        Self::of(AnimKind::FadeIn {
            id,
            shift,
            scale_from: 0.0,
            spin: 0.0,
        })
    }

    /// 创建 SpinIn 动画（manim `SpinInFromNothing` 对应）：从中心 0 缩放 +
    /// 旋转 `angle` 弧度余量进入（正 = 视觉逆时针）。
    pub fn spin_in(id: ItemId, angle: f64) -> Self {
        Self::of(AnimKind::FadeIn {
            id,
            shift: Vec2::ZERO,
            scale_from: 0.0,
            spin: angle,
        })
    }

    /// 创建 FadeOut 动画：淡出并隐藏。
    pub fn fade_out(id: ItemId) -> Self {
        Self::of(AnimKind::FadeOut(id))
    }

    /// 创建 Transform 动画：`from` 形变为 `to`。
    pub fn transform(from: ItemId, to: ItemId) -> Self {
        Self::of(AnimKind::Transform { from, to })
    }

    /// 创建 TransformTo 动画：对象变换插值到 `to`（相机推拉摇移与目标位姿的通用词汇）。
    pub fn transform_to(id: ItemId, to: Affine) -> Self {
        Self::of(AnimKind::TransformTo {
            id,
            from: Affine::IDENTITY, // 由 Timeline::play 填充（构建期当前变换）
            to,
        })
    }

    /// 位姿动作：世界系平移（manim `.animate.shift(dx, dy)` 对应）。
    pub fn shift_to(id: ItemId, dx: f64, dy: f64) -> Self {
        Self::of(AnimKind::Pose {
            id,
            delta: PoseDelta::Shift(Vec2::new(dx, dy)),
        })
    }

    /// 位姿动作：移动到目标点（对象 bbox 中心对齐）。
    pub fn move_to(id: ItemId, target: impl Into<Point>) -> Self {
        Self::of(AnimKind::Pose {
            id,
            delta: PoseDelta::MoveTo(target.into()),
        })
    }

    /// 位姿动作：绕 bbox 中心旋转（弧度；正 = 视觉逆时针，manim 语义）。
    pub fn rotate_to(id: ItemId, angle: f64) -> Self {
        Self::of(AnimKind::Pose {
            id,
            delta: PoseDelta::Rotate(angle),
        })
    }

    /// 位姿动作：绕 bbox 中心缩放。
    pub fn scale_to(id: ItemId, factor: f64) -> Self {
        Self::of(AnimKind::Pose {
            id,
            delta: PoseDelta::Scale(factor),
        })
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
