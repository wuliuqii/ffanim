//! 时间轴构建期：`Timeline` 命令式记录 `play/wait/overlap`，对象注册后得到
//! 基准快照，动画编译为"起始快照 → 目标快照"的静态调度表（[`ScheduledAnim`]）。
//!
//! 对应 manim 的 `Scene.play()`。构建期唯一的可变状态在此；
//! 编译产物 [`crate::eval::CompiledTimeline`] 不可变，渲染期纯函数查表求值。
//!
//! 两阶段模型见 docs/02-架构设计.md §3.2（全项目最核心决策）。

use crate::anim::{Anim, AnimKind};
use crate::eval::CompiledTimeline;
use crate::tracker::{TrackerId, Updater};
use crate::{Color, Item, ItemId};

/// 时间轴构建器（构建期唯一入口）。
#[derive(Debug, Clone)]
pub struct Timeline {
    fps: u32,
    cursor_s: f64,
    /// 由 [`Timeline::overlap`] 设置：下一个 play 提前量（秒）。
    pending_overlap_s: Option<f64>,
    items: Vec<Item>,
    anims: Vec<(f64, Anim)>, // (start_s, anim)
    /// 值跟踪器初始值（下标即 [`TrackerId`]）。
    trackers: Vec<f64>,
    /// 值跟踪器构建期当前值（随 [`Anim::set_value`] 推演，作为下一段的 from）。
    tracker_cur: Vec<f64>,
    /// 求值期信号（渲染期按帧重建形状）。
    updaters: Vec<Updater>,
    _canvas: CanvasArgs,
}

impl Timeline {
    /// 创建时间轴。画布参数在此传入，[`Timeline::compile`] 后不可变。
    pub fn new(width: u32, height: u32, fps: u32, background: Color) -> Self {
        Self {
            fps: fps.max(1),
            cursor_s: 0.0,
            pending_overlap_s: None,
            items: Vec::new(),
            anims: Vec::new(),
            trackers: Vec::new(),
            tracker_cur: Vec::new(),
            updaters: Vec::new(),
            _canvas: CanvasArgs {
                width,
                height,
                background,
            },
        }
    }

    /// 注册对象基准状态，返回句柄（id）。注册后即成为时间轴的一部分；
    /// 是否可见由动画决定（无任何动画的对象从第 0 帧可见）。
    pub fn add(&mut self, item: Item) -> ItemId {
        let id = self.items.len() as ItemId;
        self.items.push(Item { id, ..item });
        id
    }

    /// 注册值跟踪器（隐形，仅作为 updater 的信号源），返回句柄。
    pub fn add_tracker(&mut self, initial: f64) -> TrackerId {
        self.trackers.push(initial);
        self.tracker_cur.push(initial);
        self.trackers.len() - 1
    }

    /// 注册求值期信号：`f` 把依赖 tracker 的值映射为对象形状，每帧重建。
    ///
    /// 基准对象以 tracker 初始值生成（因此 [`Anim::create`] 的生长、
    /// [`Anim::fade_in`] 等常规动画词汇照常作用于重建后的形状）。
    pub fn add_updater(
        &mut self,
        deps: &[TrackerId],
        f: impl Fn(&[f64]) -> Item + Send + Sync + 'static,
    ) -> ItemId {
        let initial: Vec<f64> = deps.iter().map(|&t| self.trackers[t]).collect();
        let item = self.add(f(&initial));
        self.updaters.push(Updater::new(item, deps.to_vec(), f));
        item
    }

    /// 在当前游标处排入一个动画，游标推进其时长。
    pub fn play(&mut self, mut anim: Anim) -> &mut Self {
        let overlap = self.pending_overlap_s.take().unwrap_or(0.0);
        // SetValue 的 from 推演：取构建期 tracker 当前值（顺序语义，并行段需留意）
        if let AnimKind::SetValue { tracker, from, to } = &mut anim.kind {
            *from = self.tracker_cur[*tracker];
            self.tracker_cur[*tracker] = *to;
        }
        let start_s = (self.cursor_s - overlap).max(0.0);
        self.anims.push((start_s, anim.clone()));
        self.cursor_s = start_s + anim.duration_s;
        self
    }

    /// 停留（游标推进）。
    pub fn wait(&mut self, seconds: f64) -> &mut Self {
        self.cursor_s += seconds.max(0.0);
        self
    }

    /// 下一个 play 与当前游标尾部重叠（交叉过渡），游标回退相应时长。
    pub fn overlap(&mut self, seconds: f64) -> &mut Self {
        self.pending_overlap_s = Some(seconds.max(0.0));
        self
    }

    /// 编译为不可变的渲染期时间轴。
    pub fn compile(self) -> CompiledTimeline {
        let fps = f64::from(self.fps);
        let to_frame = |s: f64| (s * fps).round() as usize;
        let mut anims: Vec<ScheduledAnim> = self
            .anims
            .into_iter()
            .map(|(start_s, anim)| ScheduledAnim {
                start: to_frame(start_s),
                end: to_frame(start_s + anim.duration_s).max(to_frame(start_s) + 1),
                kind: anim.kind,
                rate: anim.rate,
            })
            .collect();
        anims.sort_by_key(|a| a.start);
        let duration_frames = anims
            .iter()
            .map(|a| a.end)
            .chain(std::iter::once(to_frame(self.cursor_s)))
            .max()
            .unwrap_or(0);
        let CanvasArgs {
            width,
            height,
            background,
        } = self._canvas;
        // 埋点：编译摘要（info）+ 逐动画调度表（debug）
        log::info!(
            "时间轴编译完成: {} 个对象 / {} 个动画 / {duration_frames} 帧 / 画布 {width}x{height}",
            self.items.len(),
            anims.len()
        );
        for a in &anims {
            log::debug!("调度: [{:>4}..{:<4}] {}", a.start, a.end, a.kind.describe());
        }
        CompiledTimeline {
            fps: self.fps,
            width,
            height,
            background,
            items: self.items,
            anims,
            trackers: self.trackers,
            updaters: self.updaters,
            duration_frames,
        }
    }
}

#[derive(Debug, Clone)]
struct CanvasArgs {
    width: u32,
    height: u32,
    background: Color,
}

/// 已调度的动画（帧为单位）。
#[derive(Debug, Clone)]
pub struct ScheduledAnim {
    /// 起始帧（含）。
    pub start: usize,
    /// 结束帧（不含）。
    pub end: usize,
    /// 动画类型（做什么）。
    pub kind: AnimKind,
    /// 缓动函数（进度 → 0..1）。
    pub rate: crate::rate::RateFunc,
}

impl ScheduledAnim {
    /// 动画在帧 f 的进度（已应用缓动），不活跃返回 None。
    pub(crate) fn alpha_at(&self, f: usize) -> Option<f64> {
        (f >= self.start && f < self.end).then(|| {
            self.rate
                .eval((f - self.start) as f64 / (self.end - self.start) as f64)
        })
    }
}
