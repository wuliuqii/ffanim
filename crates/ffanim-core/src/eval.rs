//! 渲染期求值：[`CompiledTimeline`]（不可变查表）与 [`render_frame`]（纯函数）。
//!
//! frame → 该帧所有可见对象的最终状态，无运行时闭包、无可变状态，
//! 天然可并行。这是两阶段模型的"渲染期"半边。

use crate::anim::AnimKind;
use crate::path_util::{lerp_affine, lerp_path, partial_path};
use crate::rate::lerp;
use crate::timeline::ScheduledAnim;
use crate::tracker::TrackerId;
use crate::{Color, DrawList, Item, ItemId, Paint, Shape};
use kurbo::Shape as _;
use kurbo::{Affine, Vec2}; // bounding_box

/// 编译后的不可变时间轴（渲染期唯一输入）。
pub struct CompiledTimeline {
    pub fps: u32,
    pub width: u32,
    pub height: u32,
    pub background: Color,
    /// 全部对象的基准状态，下标即 [`ItemId`]。
    pub items: Vec<Item>,
    pub anims: Vec<ScheduledAnim>,
    /// 值跟踪器初始值，下标即 [`TrackerId`]。
    pub trackers: Vec<f64>,
    /// 求值期信号（渲染期按帧重建形状）。
    pub updaters: Vec<crate::tracker::Updater>,
    pub duration_frames: usize,
}

// Updater 含 Arc<dyn Fn>，Debug/Clone 手动实现（Arc 语义下克隆共享函数）
impl Clone for CompiledTimeline {
    fn clone(&self) -> Self {
        Self {
            fps: self.fps,
            width: self.width,
            height: self.height,
            background: self.background,
            items: self.items.clone(),
            anims: self.anims.clone(),
            trackers: self.trackers.clone(),
            updaters: self.updaters.clone(),
            duration_frames: self.duration_frames,
        }
    }
}

impl std::fmt::Debug for CompiledTimeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledTimeline")
            .field("fps", &self.fps)
            .field("size", &(self.width, self.height))
            .field("items", &self.items.len())
            .field("anims", &self.anims.len())
            .field("trackers", &self.trackers)
            .field("updaters", &self.updaters)
            .field("duration_frames", &self.duration_frames)
            .finish()
    }
}

impl CompiledTimeline {
    /// `Duration::FromAudio` 语义：时间轴时长至少延伸到指定帧数。
    ///
    /// 动画结束后超出部分渲染为尾帧定格（历史段语义：动画停在终值），
    /// 用于按音频长度定片长（配解说等场景）。只延长不截短。
    pub fn with_min_duration(mut self, frames: usize) -> Self {
        self.duration_frames = self.duration_frames.max(frames);
        self
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
        // 求值期信号先于动画：updater 按依赖 tracker 值重建基准形状，
        // Create 生长 / FadeIn 等动画词汇照常作用在重建结果上
        if let Some(up) = tl.updaters.iter().find(|u| u.item_id == id) {
            let values: Vec<f64> = up
                .deps
                .iter()
                .map(|&t| tracker_value(tl, t, frame))
                .collect();
            state.shape = up.eval(&values).shape;
        }
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

/// 值跟踪器在帧 f 的值：活跃段插值，历史段取终值（确定性纯函数；段间不重叠）。
fn tracker_value(tl: &CompiledTimeline, tracker: TrackerId, f: usize) -> f64 {
    let mut v = tl.trackers[tracker];
    for anim in &tl.anims {
        let AnimKind::SetValue {
            tracker: t,
            from,
            to,
        } = &anim.kind
        else {
            continue;
        };
        if *t != tracker {
            continue;
        }
        if let Some(a) = anim.alpha_at(f) {
            return lerp(*from, *to, a);
        }
        if f >= anim.end {
            v = *to;
        }
    }
    v
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
    if appear.is_some_and(|s| f < s) {
        return false;
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
    if let (Shape::Path(pa), Shape::Path(pb)) = (&a.shape, &b.shape)
        && let Some(path) = lerp_path(pa, pb, t)
    {
        return vec![Item {
            id: a.id,
            shape: Shape::Path(path),
            paint,
            transform,
        }];
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
    use crate::anim::Anim;
    use crate::path_util::partial_path;
    use crate::rate::RateFunc;
    use crate::timeline::Timeline;
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

    // ---- M4-2：ValueTracker 值动画 + 求值期 updater ----

    fn vline(height: f64) -> Item {
        let mut p = kurbo::BezPath::new();
        p.move_to((10.0, 90.0));
        p.line_to((10.0, 90.0 - height));
        Item::from_path(p)
    }

    #[test]
    fn tracker值动画与from推演() {
        let mut tl = make_tl();
        let t = tl.add_tracker(1.0);
        tl.play(Anim::set_value(t, 0.5).during(1.0).rate(RateFunc::Linear));
        tl.play(Anim::set_value(t, 2.0).during(1.0).rate(RateFunc::Linear));
        let c = tl.compile();

        // from 推演：第二段 from = 第一段 to
        assert_eq!(
            c.anims[0].kind.describe(),
            "SetValue(tracker#0: 1.000→0.500)"
        );
        assert_eq!(
            c.anims[1].kind.describe(),
            "SetValue(tracker#0: 0.500→2.000)"
        );

        // 值查表：段前取初值 / 活跃段线性插值 / 段后取终值
        assert_eq!(tracker_value(&c, t, 0), 1.0);
        assert!((tracker_value(&c, t, 15) - 0.75).abs() < 1e-9);
        assert_eq!(tracker_value(&c, t, 30), 0.5);
        assert!((tracker_value(&c, t, 45) - 1.25).abs() < 1e-9);
        assert_eq!(tracker_value(&c, t, 90), 2.0);
    }

    #[test]
    fn updater每帧按tracker值重建() {
        let mut tl = make_tl();
        let t = tl.add_tracker(1.0);
        let id = tl.add_updater(&[t], |v| vline(v[0] * 40.0));
        tl.play(Anim::create(id).during(1.0).rate(RateFunc::Linear));
        tl.play(Anim::set_value(t, 2.0).during(1.0).rate(RateFunc::Linear));
        tl.wait(0.5);
        let c = tl.compile();

        // 基准对象以初始值生成：高度 = 1.0 * 40
        let Shape::Path(base) = &c.items[0].shape else {
            panic!()
        };
        let Some(kurbo::PathEl::LineTo(base_end)) = base.elements().get(1) else {
            panic!()
        };

        // create 已结束、tracker 半程（1.0→2.0 的线性中点 = 1.5）→ 高度 60
        let mid = render_frame(&c, 45);
        let Shape::Path(p) = &mid.items[0].shape else {
            panic!()
        };
        let Some(kurbo::PathEl::LineTo(end)) = p.elements().get(1) else {
            panic!()
        };
        assert!(
            (end.y - (90.0 - 60.0)).abs() < 1e-9,
            "应为 tracker=1.5 的高度"
        );

        // 结束后：tracker = 2.0 → 高度 80（不同于基准，证明每帧重建）
        let Shape::Path(p) = &render_frame(&c, 90).items[0].shape else {
            panic!()
        };
        let Some(kurbo::PathEl::LineTo(end)) = p.elements().get(1) else {
            panic!()
        };
        assert!((end.y - (90.0 - 80.0)).abs() < 1e-9);
        assert!((end.y - base_end.y).abs() > 1.0);
    }

    #[test]
    fn updater叠加create生长() {
        let mut tl = make_tl();
        let t = tl.add_tracker(1.0);
        let id = tl.add_updater(&[t], |v| vline(v[0] * 40.0));
        tl.play(Anim::create(id).during(1.0).rate(RateFunc::Linear));
        let c = tl.compile();

        // create 半程：tracker 恒为初值 1.0（高度 40），但路径只画出一半
        let mid = render_frame(&c, 15);
        let Shape::Path(full) = &c.items[0].shape else {
            panic!()
        };
        let Shape::Path(part) = &mid.items[0].shape else {
            panic!()
        };
        assert!(part.elements().len() < full.elements().len() + 2);
        // 半程线段终点在 10..50 之间（起点 90、终点 50）
        let Some(kurbo::PathEl::LineTo(end)) = part.elements().last() else {
            panic!()
        };
        assert!(end.y > 50.0 && end.y < 90.0);
    }
}
