//! 时间轴两阶段模型（docs/02-架构设计.md §3.2，全项目最核心决策）：
//!
//! - **构建期**：`Timeline` 命令式记录 `play/wait/overlap`，对象注册后得到
//!   基准快照，动画编译为"起始快照 → 目标快照"的静态调度表；
//! - **渲染期**：[`render_frame`] 是纯函数，frame → 该帧所有可见对象的
//!   最终状态，无运行时闭包、无可变状态，天然可并行。
//!
//! 对应 manim 的对应物：`Scene.play()` → [`Timeline::play`]，
//! `Create/FadeIn/FadeOut/Transform` → [`Anim`]，`rate_func` → [`RateFunc`]。

use crate::path_util::{lerp_affine, lerp_path, partial_path};
use crate::rate::{RateFunc, lerp};
use crate::{Color, DrawList, Item, ItemId, Paint, Shape};
use kurbo::Shape as _;
use kurbo::{Affine, Vec2}; // bounding_box

// ---------------------------------------------------------------------------
// 构建期 API
// ---------------------------------------------------------------------------

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

/// 时间轴构建器（构建期唯一入口）。
#[derive(Debug, Clone)]
pub struct Timeline {
    fps: u32,
    cursor_s: f64,
    /// 由 [`Timeline::overlap`] 设置：下一个 play 提前量（秒）。
    pending_overlap_s: Option<f64>,
    items: Vec<Item>,
    anims: Vec<(f64, Anim)>, // (start_s, anim)
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

    /// 在当前游标处排入一个动画，游标推进其时长。
    pub fn play(&mut self, anim: Anim) -> &mut Self {
        let overlap = self.pending_overlap_s.take().unwrap_or(0.0);
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

// ---------------------------------------------------------------------------
// 渲染期（纯函数）
// ---------------------------------------------------------------------------

/// 编译后的不可变时间轴（渲染期唯一输入）。
#[derive(Debug, Clone)]
pub struct CompiledTimeline {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub background: Color,
    /// 全部对象的基准状态，下标即 [`ItemId`]。
    pub items: Vec<Item>,
    pub anims: Vec<ScheduledAnim>,
    pub duration_frames: usize,
}

/// 已调度的动画（帧为单位）。
#[derive(Debug, Clone)]
pub struct ScheduledAnim {
    pub start: usize,
    pub end: usize,
    pub kind: AnimKind,
    pub rate: RateFunc,
}

impl ScheduledAnim {
    /// 动画在帧 f 的进度（已应用缓动），不活跃返回 None。
    fn alpha_at(&self, f: usize) -> Option<f64> {
        (f >= self.start && f < self.end).then(|| {
            self.rate
                .eval((f - self.start) as f64 / (self.end - self.start) as f64)
        })
    }
}

/// 求值一帧：frame → DrawList（确定性纯函数）。
pub fn render_frame(tl: &CompiledTimeline, frame: usize) -> DrawList {
    let mut draw = DrawList::new(tl.width, tl.height, tl.background);

    // 活跃的 Transform：from/to 两个对象跳过常规通道，由形变产物替代
    let active_morphs: Vec<(&ScheduledAnim, f64)> = tl
        .anims
        .iter()
        .filter(|a| matches!(a.kind, AnimKind::Transform { .. }))
        .filter_map(|a| a.alpha_at(frame).map(|t| (a, t)))
        .collect();
    let mut skipped: std::collections::HashSet<ItemId> = std::collections::HashSet::new();
    for (anim, _) in &active_morphs {
        if let AnimKind::Transform { from, to } = anim.kind {
            skipped.insert(from);
            skipped.insert(to);
        }
    }

    for item in &tl.items {
        let id = item.id;
        if skipped.contains(&id) || !is_visible(tl, id, frame) {
            continue;
        }
        let mut state = item.clone();
        let mut opacity = 1.0;
        for anim in &tl.anims {
            let Some(alpha) = anim.alpha_at(frame) else {
                continue;
            };
            match &anim.kind {
                AnimKind::Create(create_id) if *create_id == id => {
                    apply_create(&mut state, alpha);
                    opacity *= alpha; // 填充随进度淡入
                }
                AnimKind::FadeIn { id: in_id, shift } if *in_id == id => {
                    opacity *= alpha;
                    state.transform = enter_transform(&state, *shift, alpha) * state.transform;
                }
                AnimKind::FadeOut(out_id) if *out_id == id => {
                    opacity *= 1.0 - alpha;
                }
                _ => {}
            }
        }
        state.paint = state.paint.with_opacity(opacity);
        // 完全透明的对象不出现在绘制列表（也避免空路径进入渲染层）
        if opacity > 1e-6 {
            draw.push(state);
        }
    }

    // 形变产物
    for (anim, t) in active_morphs {
        if let AnimKind::Transform { from, to } = anim.kind {
            for morphed in morph_items(&tl.items[from as usize], &tl.items[to as usize], t) {
                draw.push(morphed);
            }
        }
    }
    // 埋点：逐帧求值结果（trace 级，动画同步排查用）
    log::trace!(
        "render_frame 求值: frame={frame}, visible={}",
        draw.items.len()
    );
    draw
}

/// 对象在帧 f 是否可见（出场/退场/形变交换语义）。
fn is_visible(tl: &CompiledTimeline, id: ItemId, f: usize) -> bool {
    let mut appear: Option<usize> = None; // None = 第 0 帧可见
    let mut hidden_before: Vec<usize> = Vec::new(); // 该帧起不可见
    for anim in &tl.anims {
        match &anim.kind {
            AnimKind::Create(x) | AnimKind::FadeIn { id: x, .. } if *x == id => {
                appear = Some(appear.map_or(anim.start, |s: usize| s.min(anim.start)));
            }
            AnimKind::FadeOut(x) if *x == id => hidden_before.push(anim.end),
            AnimKind::Transform { from, to } => {
                if *from == id {
                    hidden_before.push(anim.end);
                }
                if *to == id {
                    appear = Some(appear.map_or(anim.start, |s: usize| s.min(anim.start)));
                }
            }
            _ => {}
        }
    }
    if let Some(s) = appear {
        if f < s {
            return false;
        }
    }
    // 每个隐藏点之后，允许其后的重新出场动画复活对象
    for &h in &hidden_before {
        if f < h {
            continue;
        }
        let revived = tl.anims.iter().any(|anim| {
            let start = anim.start;
            start >= h
                && start <= f
                && matches!(
                    &anim.kind,
                    AnimKind::Create(x) | AnimKind::FadeIn { id: x, .. } if *x == id
                )
        });
        if !revived {
            return false;
        }
    }
    true
}

/// Create：描边路径按进度生长。
fn apply_create(state: &mut Item, alpha: f64) {
    if let Shape::Path(path) = &state.shape {
        state.shape = Shape::Path(partial_path(path, alpha));
    }
}

/// FadeIn 的进入变换：从 shift 偏移处、90% 缩放滑入。
fn enter_transform(state: &Item, shift: Vec2, alpha: f64) -> Affine {
    let scale = 0.9 + 0.1 * alpha;
    let center = item_center(state);
    // 以中心为基准缩放
    let about_center = Affine::translate(center.to_vec2())
        * Affine::scale(scale)
        * Affine::translate(-center.to_vec2());
    Affine::translate(shift * (1.0 - alpha)) * about_center
}

/// 对象包围盒中心（变换后）。
fn item_center(state: &Item) -> kurbo::Point {
    let Shape::Path(path) = &state.shape else {
        return kurbo::Point::ZERO;
    };
    let b = path.bounding_box();
    let c = kurbo::Point::new((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0);
    state.transform * c
}

/// Transform 求值：元素序列一致 → 单个形变对象；否则 → 交叉淡化对。
fn morph_items(a: &Item, b: &Item, t: f64) -> Vec<Item> {
    let transform = lerp_affine(&a.transform, &b.transform, t);
    let paint = lerp_paint(&a.paint, &b.paint, t);
    if let (Shape::Path(pa), Shape::Path(pb)) = (&a.shape, &b.shape) {
        if let Some(path) = lerp_path(pa, pb, t) {
            return vec![Item {
                id: a.id,
                shape: Shape::Path(path),
                paint,
                transform,
            }];
        }
    }
    // 退化：交叉淡化（过滤完全透明项）
    let fade_a = Item {
        paint: a.paint.with_opacity(1.0 - t),
        ..a.clone()
    };
    let fade_b = Item {
        paint: b.paint.with_opacity(t),
        ..b.clone()
    };
    let mut out = Vec::with_capacity(2);
    if t < 1.0 {
        out.push(fade_a);
    }
    if t > 0.0 {
        out.push(fade_b);
    }
    out
}

/// 填充/描边颜色插值（一侧为 None 时取另一侧）。
fn lerp_paint(a: &Paint, b: &Paint, t: f64) -> Paint {
    Paint {
        fill: match (a.fill, b.fill) {
            (Some(x), Some(y)) => Some(Color::lerp(x, y, t)),
            (x, None) => x,
            (None, y) => y,
        },
        stroke: match (a.stroke, b.stroke) {
            (Some(x), Some(y)) => Some(Color::lerp(x, y, t)),
            (x, None) => x,
            (None, y) => y,
        },
        stroke_width: lerp(a.stroke_width, b.stroke_width, t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_util::partial_path;
    use kurbo::{Circle, Rect};

    fn make_tl() -> Timeline {
        Timeline::new(100, 100, 30, Color::new(0, 0, 0, 255))
    }

    fn rect_item(fill: Color) -> Item {
        Item::rect(Rect::new(0.0, 0.0, 40.0, 40.0)).with_fill(fill)
    }

    #[test]
    fn 无动画对象全程可见() {
        let mut tl = make_tl();
        tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.wait(1.0);
        let compiled = tl.compile();
        assert_eq!(compiled.duration_frames, 30);
        assert_eq!(render_frame(&compiled, 0).items.len(), 1);
        assert_eq!(render_frame(&compiled, 29).items.len(), 1);
    }

    #[test]
    fn create生命周期() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.play(Anim::create(id).during(1.0));
        tl.wait(1.0);
        let c = tl.compile();

        let base_len = match &c.items[0].shape {
            Shape::Path(p) => p.elements().len(),
            _ => panic!(),
        };

        // 创建前不可见
        assert_eq!(render_frame(&c, 0).items.len(), 0);
        // 中途：可见且路径不完整
        let mid = render_frame(&c, 15);
        assert_eq!(mid.items.len(), 1);
        let Shape::Path(p) = &mid.items[0].shape else {
            panic!()
        };
        assert!(p.elements().len() < base_len, "中途路径应不完整");
        // 结束后可见且路径完整
        let done = render_frame(&c, 45);
        let Shape::Path(p) = &done.items[0].shape else {
            panic!()
        };
        assert_eq!(p.elements().len(), base_len);
    }

    #[test]
    fn fadein_fadeout透明度() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.play(Anim::fade_in(id).during(1.0));
        tl.wait(1.0);
        tl.play(Anim::fade_out(id).during(1.0));
        let c = tl.compile();

        let mid_in = render_frame(&c, 15);
        assert_eq!(mid_in.items[0].paint.fill.unwrap().a, 128);
        // fade_out 从 2.0s（第 60 帧）开始
        let mid_out = render_frame(&c, 75);
        assert_eq!(mid_out.items[0].paint.fill.unwrap().a, 128);
        // 淡出完成后隐藏
        assert_eq!(render_frame(&c, 90).items.len(), 0);
    }

    #[test]
    fn transform生命周期与形变() {
        let mut tl = make_tl();
        let a = tl
            .add(Item::rect(Rect::new(0.0, 0.0, 40.0, 40.0)).with_fill(Color::new(255, 0, 0, 255)));
        let b = tl.add(
            Item::circle(Circle::new((50.0, 50.0), 30.0)).with_fill(Color::new(0, 0, 255, 255)),
        );
        tl.wait(1.0); // 两者先静态可见
        tl.play(Anim::transform(a, b).during(1.0));
        tl.wait(1.0);
        let c = tl.compile();

        // 形变前：矩形可见，目标圆形尚不可见（出场帧 = 形变开始帧）
        let before = render_frame(&c, 15);
        assert_eq!(before.items.len(), 1);
        assert_eq!(before.items[0].id, a);
        // 形变中：常规通道跳过，产出生形变项（矩形→圆形元素序列不同 → 交叉淡化 2 项）
        let mid = render_frame(&c, 45);
        assert_eq!(mid.items.len(), 2, "交叉淡化应产生两项");
        // 形变后：from 消失，to 可见
        let after = render_frame(&c, 75);
        assert_eq!(after.items.len(), 1);
        assert_eq!(after.items[0].id, b);
    }

    #[test]
    fn overlap调度() {
        let mut tl = make_tl();
        let a = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        let b = tl.add(rect_item(Color::new(0, 255, 0, 255)));
        tl.play(Anim::create(a).during(1.0));
        tl.overlap(0.5); // b 提前 0.5s 开始
        tl.play(Anim::create(b).during(1.0));
        let c = tl.compile();
        let starts: Vec<usize> = c.anims.iter().map(|x| x.start).collect();
        assert_eq!(starts, vec![0, 15]);
        assert_eq!(c.duration_frames, 45);
    }

    #[test]
    fn 渲染确定性() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.play(Anim::create(id).during(1.0));
        let c = tl.compile();
        let d1 = render_frame(&c, 10);
        let d2 = render_frame(&c, 10);
        assert_eq!(d1.items[0].paint.fill, d2.items[0].paint.fill);
        assert!(
            partial_path(&kurbo::BezPath::new(), 0.5)
                .elements()
                .is_empty()
        );
    }
}
