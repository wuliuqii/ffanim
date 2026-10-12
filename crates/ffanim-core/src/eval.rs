//! 渲染期求值：[`CompiledTimeline`]（不可变查表）与 [`render_frame`]（纯函数）。
//!
//! frame → 该帧所有可见对象的最终状态，无运行时闭包、无可变状态，
//! 天然可并行。这是两阶段模型的"渲染期"半边。

use crate::anim::AnimKind;
use crate::path_util::{lerp_affine, lerp_path, partial_path, sample_path};
use crate::rate::lerp;
use crate::timeline::ScheduledAnim;
use crate::tracker::TrackerId;
use crate::{Color, DrawList, Item, ItemId, Paint, Shape};
use kurbo::Shape as _;
use kurbo::{Affine, BezPath, Vec2}; // bounding_box

/// 编译后的不可变时间轴（渲染期唯一输入）。
pub struct CompiledTimeline {
    /// 帧率。
    pub fps: u32,
    /// 画布宽（像素）。
    pub width: u32,
    /// 画布高（像素）。
    pub height: u32,
    /// 背景色。
    pub background: Color,
    /// 全部对象的基准状态，下标即 [`ItemId`]。
    pub items: Vec<Item>,
    /// 静态调度表（起始快照 → 目标快照），按 start_s 升序。
    pub anims: Vec<ScheduledAnim>,
    /// 值跟踪器初始值，下标即 [`TrackerId`]。
    pub trackers: Vec<f64>,
    /// 求值期信号（渲染期按帧重建形状）。
    pub updaters: Vec<crate::tracker::Updater>,
    /// 相机对象句柄（None = 无相机，世界系即屏幕系）。
    pub camera: Option<ItemId>,
    /// 总帧数（duration_s × fps）。
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
            camera: self.camera,
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
            .field("camera", &self.camera)
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

    // 相机：world→screen 仿射逐帧求值（None = 单位阵）；相机对象本身不进绘制列表
    let cam = match tl.camera {
        Some(cid) => transform_at(tl, cid, tl.items[cid as usize].transform, frame),
        None => Affine::IDENTITY,
    };

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
        if Some(id) == tl.camera {
            continue; // 相机即 Item 但不可见（docs/02 §3.4）
        }
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
        // TransformTo 段求值：段中插值，段前取 from（段后取存储基线 = 目标）
        state.transform = transform_at(tl, id, state.transform, frame);
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
                AnimKind::Uncreate(uc_id) if *uc_id == id => {
                    apply_create(&mut state, 1.0 - alpha); // 路径按进度收回
                    opacity *= 1.0 - alpha; // 填充随进度淡出
                }
                AnimKind::FadeIn {
                    id: in_id,
                    shift,
                    scale_from,
                    spin,
                } if *in_id == id => {
                    opacity *= alpha;
                    state.transform = enter_transform(&state, *shift, *scale_from, *spin, alpha)
                        * state.transform;
                }
                AnimKind::FadeOut(out_id) if *out_id == id => {
                    opacity *= 1.0 - alpha;
                }
                _ => {}
            }
        }
        with_opacity_all(&mut state, opacity);
        // 完全透明的对象不出现在绘制列表（也避免空路径进入渲染层）
        if opacity > 1e-6 {
            state.transform = cam * state.transform;
            draw.push(state);
        }
    }

    // 形变产物
    for (anim, t) in active_morphs {
        if let AnimKind::Transform { from, to } = anim.kind {
            for mut morphed in morph_items(&tl.items[from as usize], &tl.items[to as usize], t) {
                morphed.transform = cam * morphed.transform;
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

/// 对象在帧 f 的变换：变换类动画（TransformTo / MoveAlongPath）按调度表求值。
///
/// 每段动画的三相语义：
/// - 活跃段（start ≤ f < end）：插值/采样；
/// - 段前（f < start）：取该段 from（基线已被构建期快进到 to，段前须回起点）——
///   **仅对该对象首个生效动画**（`touched` 标记），否则后段的 from 会覆盖
///   前段的段中插值（顺序串接场景，回归测试 `transform_at_顺序段不互相覆盖`）；
/// - 段后（f ≥ end）：基线即终态，不再覆盖。
fn transform_at(tl: &CompiledTimeline, id: ItemId, base: Affine, f: usize) -> Affine {
    let mut t = base;
    let mut touched = false; // 已有动画为该帧设定状态（活跃段或首个段前）
    for anim in &tl.anims {
        match &anim.kind {
            AnimKind::TransformTo { id: tid, from, to } if *tid == id => {
                if let Some(a) = anim.alpha_at(f) {
                    t = lerp_affine(from, to, a);
                    touched = true;
                } else if f < anim.start && !touched {
                    t = *from;
                    touched = true;
                }
            }
            AnimKind::MoveAlongPath {
                id: mid,
                path,
                base: from,
            } if *mid == id => {
                if let Some(a) = anim.alpha_at(f) {
                    // 活跃段：沿路径弧长采样位移（起点对齐）
                    t = Affine::translate(move_offset(path, a)) * *from;
                    touched = true;
                } else if f < anim.start && !touched {
                    t = *from;
                    touched = true;
                }
                // 段后：基线已快进到路径终点，保持
            }
            _ => {}
        }
    }
    t
}

/// MoveAlongPath 在进度 `h`（已缓动）的位移向量：路径采样点 − 路径起点。
fn move_offset(path: &BezPath, h: f64) -> Vec2 {
    let s0 = sample_path(path, 0.0);
    let p = sample_path(path, h);
    Vec2::new(p.x - s0.x, p.y - s0.y)
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
            AnimKind::Uncreate(x) if *x == id => hidden_before.push(anim.end),
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

/// FadeIn 的进入变换：从 shift 偏移处、`scale_from` 缩放、旋转 `spin` 余量滑入。
fn enter_transform(state: &Item, shift: Vec2, scale_from: f64, spin: f64, alpha: f64) -> Affine {
    let scale = scale_from + (1.0 - scale_from) * alpha;
    let center = item_center(state);
    // 以中心为基准缩放 + 旋转（旋转余量随 alpha 归零）
    let about_center = Affine::translate(center.to_vec2())
        * Affine::rotate(-spin * (1.0 - alpha))
        * Affine::scale(scale)
        * Affine::translate(-center.to_vec2());
    Affine::translate(shift * (1.0 - alpha)) * about_center
}

/// 对象包围盒中心（变换后；组合取子项包围盒并集）。
fn item_center(state: &Item) -> kurbo::Point {
    let mut bbox = None;
    collect_bbox(state, Affine::IDENTITY, &mut bbox);
    bbox.map_or(kurbo::Point::ZERO, |b| {
        kurbo::Point::new((b.x0 + b.x1) / 2.0, (b.y0 + b.y1) / 2.0)
    })
}

/// 递归收集对象在 `parent` 坐标系下的包围盒（None = 无可见几何）。
fn collect_bbox(item: &Item, parent: Affine, out: &mut Option<kurbo::Rect>) {
    let t = parent * item.transform;
    let local = match &item.shape {
        Shape::Path(path) => Some(path.bounding_box()),
        Shape::Image(img) => Some(kurbo::Rect::new(
            0.0,
            0.0,
            f64::from(img.width),
            f64::from(img.height),
        )),
        Shape::Group(children) => {
            let mut b = None;
            for child in children {
                collect_bbox(child, t, &mut b);
            }
            b
        }
    };
    if let Some(b) = local {
        let b = t.transform_rect_bbox(b);
        match out {
            Some(o) => *o = o.union(b),
            None => *out = Some(b),
        }
    }
}

/// 整体透明度递归应用（组合的子项同步淡入/淡出）。
fn with_opacity_all(item: &mut Item, opacity: f64) {
    item.paint = item.paint.with_opacity(opacity);
    if let Shape::Group(children) = &mut item.shape {
        for child in children {
            with_opacity_all(child, opacity);
        }
    }
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
    let mut fade_a = a.clone();
    with_opacity_all(&mut fade_a, 1.0 - t);
    let mut fade_b = b.clone();
    with_opacity_all(&mut fade_b, t);
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
        opacity: lerp(a.opacity, b.opacity, t),
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn 组合淡入传导子项() {
        let mut tl = Timeline::new(64, 64, 30, Color::new(0, 0, 0, 255));
        let group = tl.add(Item {
            id: 0,
            shape: Shape::Group(vec![
                Item::circle(kurbo::Circle::new((0.0, 0.0), 10.0))
                    .with_fill(Color::new(255, 0, 0, 255)),
            ]),
            paint: Paint::default(),
            transform: Affine::IDENTITY,
        });
        tl.play(Anim::fade_in(group).during(1.0));
        let draw = render_frame(&tl.compile(), 15);
        assert_eq!(draw.items.len(), 1);
        let Shape::Group(children) = &draw.items[0].shape else {
            panic!("应为组合");
        };
        // 子项 paint 透明度系数与颜色 alpha 同步为半程 0.5
        assert!((children[0].paint.opacity - 0.5).abs() < 1e-6);
        assert_eq!(children[0].paint.fill.unwrap().a, 128);
    }

    #[test]
    fn 图片淡入透明度() {
        let mut tl = Timeline::new(64, 64, 30, Color::new(0, 0, 0, 255));
        let img = tl.add(Item::from_image(vec![255; 4 * 4 * 4], 4, 4).unwrap());
        tl.play(Anim::fade_in(img).during(1.0));
        let ct = tl.compile();
        // 半程：透明度 0.5，图片对象在场
        let draw = render_frame(&ct, 15);
        assert_eq!(draw.items.len(), 1);
        assert!((draw.items[0].paint.opacity - 0.5).abs() < 1e-6);
    }
    use super::*;
    use crate::anim::Anim;
    use crate::camera::Camera;
    use crate::path_util::partial_path;
    use crate::rate::RateFunc;
    use crate::timeline::Timeline;
    use kurbo::{Circle, Point, Rect, Vec2};

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

    /// 探针：单位阵变换的 10x10 矩形（局部原点即世界坐标，便于断言屏幕位置）。
    fn probe(at: (f64, f64)) -> Item {
        Item::rect(Rect::new(0.0, 0.0, 10.0, 10.0)).with_transform(Affine::translate(at))
    }

    #[test]
    fn 相机推近_注视点为不动点() {
        let mut tl = make_tl(); // 100x100 @30fps
        tl.add(probe((50.0, 50.0))); // 注视点处
        tl.add(probe((25.0, 50.0))); // 注视点左侧
        let cam = tl.camera(Camera::at(100, 100));
        let target = Camera {
            center: Vec2::new(50.0, 50.0),
            zoom: 2.0,
            rotation: 0.0,
        };
        tl.play(Anim::transform_to(cam, target.affine(100, 100)).during(1.0));
        let c = tl.compile();

        // 相机对象不进绘制列表
        assert_eq!(render_frame(&c, 0).items.len(), 2);
        let origin = |f: usize, i: usize| {
            let d = render_frame(&c, f);
            d.items[i].transform * Point::ZERO
        };
        // 注视点（画布中心）全程不动
        assert_eq!(origin(0, 0), Point::new(50.0, 50.0));
        assert_eq!(origin(30, 0), Point::new(50.0, 50.0));
        // 左侧点随 zoom 2 外推：50 + (25-50)*2 = 0
        assert_eq!(origin(0, 1), Point::new(25.0, 50.0));
        assert_eq!(origin(30, 1), Point::new(0.0, 50.0));
    }

    #[test]
    fn 无相机注册时世界系即屏幕系() {
        let mut tl = make_tl();
        tl.add(probe((10.0, 20.0)));
        let c = tl.compile();
        let t = render_frame(&c, 0).items[0].transform;
        assert_eq!(t, Affine::translate((10.0, 20.0)), "无相机 = 单位阵合成");
    }

    #[test]
    fn transform_to_段前段中段后语义() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255))); // 基线 = 单位阵
        tl.play(Anim::transform_to(id, Affine::translate((90.0, 0.0))).during(1.0));
        let c = tl.compile();
        let x_of = |f: usize| render_frame(&c, f).items[0].transform.as_coeffs()[4];
        // 段前：构建期基线
        assert_eq!(x_of(0), 0.0);
        // 段中：系数插值（第 15 帧 Smooth 缓动恰为半程 45）
        let mid = x_of(15);
        assert!((40.0..50.0).contains(&mid), "段中应为插值，实际 {mid}");
        // 段后：停在目标（存储基线已快进）
        assert_eq!(x_of(45), 90.0);
    }

    #[test]
    fn group_步进错开与游标推进() {
        let mut tl = make_tl();
        let a = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        let b = tl.add(rect_item(Color::new(0, 255, 0, 255)));
        tl.wait(0.5);
        tl.group(
            [Anim::create(a).during(1.0), Anim::create(b).during(1.0)],
            0.25,
        );
        let c = tl.compile();
        let starts: Vec<usize> = c.anims.iter().map(|x| x.start).collect();
        assert_eq!(starts, vec![15, 23]); // 0.5s / 0.75s（首段 + 固定步长 0.25s）
        // 游标 = 最后一段结束：0.75 + 1.0 = 1.75s = 52.5 帧
        assert_eq!(c.duration_frames, 53);
    }

    #[test]
    fn lagged_start_间隔为段时长乘比例() {
        let mut tl = make_tl();
        let a = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        let b = tl.add(rect_item(Color::new(0, 255, 0, 255)));
        tl.lagged_start(
            [Anim::create(a).during(1.0), Anim::create(b).during(2.0)],
            0.5,
        );
        let c = tl.compile();
        let starts: Vec<usize> = c.anims.iter().map(|x| x.start).collect();
        // b 起始 = 0 + 0.5×1.0 = 0.5s（比例 × 前段时长，manim lagged_start 语义）
        assert_eq!(starts, vec![0, 15]);
        // 游标 = b 结束 = 0.5 + 2.0 = 2.5s
        assert_eq!(c.duration_frames, 75);
    }

    // ---- M9.7：P1 薄糖批（Uncreate / FadeIn 方向家族 / save_state-restore）----

    #[test]
    fn uncreate生命周期() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.play(Anim::create(id).during(1.0));
        tl.wait(1.0);
        tl.play(Anim::uncreate(id).during(1.0));
        let c = tl.compile();

        let base_len = match &c.items[0].shape {
            Shape::Path(p) => p.elements().len(),
            _ => panic!(),
        };
        // create 完成后：完整可见
        let full = render_frame(&c, 45);
        assert_eq!(full.items.len(), 1);
        // 收尾半程：路径不完整且透明度下降
        let mid = render_frame(&c, 75);
        assert_eq!(mid.items.len(), 1, "收尾中途对象在场");
        assert!(mid.items[0].paint.opacity < 0.9, "填充应随进度淡出");
        let Shape::Path(p) = &mid.items[0].shape else {
            panic!()
        };
        assert!(p.elements().len() < base_len, "中途路径应已收回一部分");
        // 段后隐藏
        assert_eq!(render_frame(&c, 105).items.len(), 0);
    }

    #[test]
    fn grow_from_center_中心缩放进入() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255))); // 40x40 @ (0,0)，中心 (20,20)
        tl.play(Anim::grow_from_center(id).during(1.0));
        let c = tl.compile();

        // 半程：缩放 0.5（绕中心 (20,20)）→ 平移系数 = 20-20×0.5 = 10
        let mid = render_frame(&c, 15);
        assert!((mid.items[0].paint.opacity - 0.5).abs() < 1e-6);
        let t = mid.items[0].transform.as_coeffs();
        assert!((t[0] - 0.5).abs() < 1e-6, "scale 系数: {t:?}");
        assert!(
            (t[4] - 10.0).abs() < 1e-6 && (t[5] - 10.0).abs() < 1e-6,
            "{t:?}"
        );
        // 段末：完全进入，与基线一致
        assert_eq!(render_frame(&c, 30).items[0].transform, Affine::IDENTITY);
    }

    #[test]
    fn spin_in_旋转进入() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.play(Anim::spin_in(id, std::f64::consts::PI).during(1.0));
        let c = tl.compile();

        // 半程：旋转余量 -π/2（视觉顺时针回正）+ 缩放 0.5 → 变换非单位阵
        let mid = render_frame(&c, 15).items[0].transform;
        assert!(mid.as_coeffs()[1].abs() > 0.1, "应有旋转分量: {mid:?}");
        // 段末：余量归零，与基线一致
        assert_eq!(render_frame(&c, 30).items[0].transform, Affine::IDENTITY);
    }

    #[test]
    fn fade_in_from_方向偏移() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        tl.play(Anim::fade_in_from(id, crate::layout::Direction::Right, 60.0).during(1.0));
        let c = tl.compile();

        // 半程：shift 余量 = 60×(1-0.5) = 30，加上 90%→100% 缩放的平移补偿
        // （绕中心 (20,20) 缩放 0.95 → tx 补 20×0.05 = 1.0）→ 合计 31
        let mid = render_frame(&c, 15).items[0].transform.as_coeffs();
        assert!((mid[4] - 31.0).abs() < 1e-6, "{mid:?}");
    }

    // ---- M9.10：P1-4 MoveAlongPath + transform_at 顺序段根修 ----

    /// 回归：顺序两段 TransformTo 的段中/段前求值不得被后段 from 覆盖
    ///（存量 bug：`f < start → from` 无条件覆盖，段中插值与段前状态全错）。
    #[test]
    fn transform_at_顺序段不互相覆盖() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255))); // 基线 = 单位阵
        tl.play(
            Anim::shift_to(id, 90.0, 0.0)
                .during(1.0)
                .rate(RateFunc::Linear),
        );
        tl.play(
            Anim::shift_to(id, 90.0, 80.0)
                .during(1.0)
                .rate(RateFunc::Linear),
        );
        let c = tl.compile();
        let xy_of = |f: usize| {
            let t = render_frame(&c, f).items[0].transform.as_coeffs();
            (t[4], t[5])
        };
        // 段前（第 0 帧）：应为基线原点，而非后段 from（90,0）
        assert_eq!(xy_of(0), (0.0, 0.0), "段前应取首段 from（基线）");
        // 前段中点：线性插值 (45, 0)
        assert_eq!(xy_of(15), (45.0, 0.0), "前段段中应插值");
        // 前段末帧：(87, 0)
        assert_eq!(xy_of(29), (87.0, 0.0));
        // 后段中点：shift_to 相对前段终点 (90,0) 平移 (90,80) → to=(180,80)，
        // 插值半程 = (135, 40)
        assert_eq!(xy_of(45), (135.0, 40.0), "后段段中应插值");
        // 段后：驻停终点（基线已快进）
        assert_eq!(xy_of(60), (180.0, 80.0));
    }

    fn line_path(from: (f64, f64), to: (f64, f64)) -> kurbo::BezPath {
        let mut p = kurbo::BezPath::new();
        p.move_to(from);
        p.line_to(to);
        p
    }

    #[test]
    fn move_along_path_直线匀速与三相() {
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255))); // 基线 = 单位阵
        tl.wait(0.5); // 段前留白：验证段前不移动
        // 轨迹 (10,20)→(110,60)：位移相对路径起点；线性 1s @30fps
        tl.play(
            Anim::move_along_path(id, &line_path((10.0, 20.0), (110.0, 60.0)))
                .during(1.0)
                .rate(RateFunc::Linear),
        );
        tl.wait(0.5); // 段后驻停
        let c = tl.compile();
        let xy_of = |f: usize| {
            let t = render_frame(&c, f).items[0].transform.as_coeffs();
            (t[4], t[5])
        };
        // 段前：位移 0（起点对齐语义——对象停在初始位置）
        assert_eq!(xy_of(0), (0.0, 0.0));
        assert_eq!(xy_of(14), (0.0, 0.0));
        // 段中 1/2（第 15 帧，窗口 15..45）：弧长半程 = (50, 20)
        assert_eq!(xy_of(30), (50.0, 20.0), "段中应为弧长半程位移");
        // 窗口末帧（第 44 帧，alpha = 29/30）：(96.67, 38.67)
        let t = render_frame(&c, 44).items[0].transform.as_coeffs();
        assert!((t[4] - 290.0 / 3.0).abs() < 1e-6 && (t[5] - 116.0 / 3.0).abs() < 1e-6);
        // 段后：驻停终点
        assert_eq!(xy_of(45), (100.0, 40.0));
        assert_eq!(xy_of(60), (100.0, 40.0));
        // prepare 基线推进：编译表基线 = 终点偏移（后续动画基于终点接续）
        assert_eq!(
            c.items[id as usize].transform,
            Affine::translate((100.0, 40.0))
        );
    }

    #[test]
    fn move_along_path_基线位移叠加与后续接续() {
        let mut tl = make_tl();
        // 对象有初始基线位移（30,10）：路径位移应叠加在基线上
        let id = tl.add(
            rect_item(Color::new(255, 0, 0, 255)).with_transform(Affine::translate((30.0, 10.0))),
        );
        tl.play(
            Anim::move_along_path(id, &line_path((0.0, 0.0), (40.0, 0.0)))
                .during(1.0)
                .rate(RateFunc::Linear),
        );
        // 终点接续：move 结束后 shift_to 相对"路径终点"位姿
        tl.play(
            Anim::shift_to(id, 0.0, 50.0)
                .during(1.0)
                .rate(RateFunc::Linear),
        );
        let c = tl.compile();
        let xy_of = |f: usize| {
            let t = render_frame(&c, f).items[0].transform.as_coeffs();
            (t[4], t[5])
        };
        // 段前：(30,10)；段中 1/2：(30+20, 10)；段后 move 终点：(70, 10)
        assert_eq!(xy_of(0), (30.0, 10.0));
        assert_eq!(xy_of(15), (50.0, 10.0));
        assert_eq!(xy_of(30), (70.0, 10.0));
        // 后续 shift_to 段中：(70, 35)；段后：(70, 60)——基于 move 终点而非原位
        assert_eq!(xy_of(45), (70.0, 35.0), "后续动画应基于路径终点接续");
        assert_eq!(xy_of(60), (70.0, 60.0));
    }
}
