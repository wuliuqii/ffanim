//! 时间轴构建期：`Timeline` 命令式记录 `play/wait/overlap`，对象注册后得到
//! 基准快照，动画编译为"起始快照 → 目标快照"的静态调度表（[`ScheduledAnim`]）。
//!
//! 对应 manim 的 `Scene.play()`。构建期唯一的可变状态在此；
//! 编译产物 [`crate::eval::CompiledTimeline`] 不可变，渲染期纯函数查表求值。
//!
//! 两阶段模型见 docs/02-架构设计.md §3.2（全项目最核心决策）。

use crate::anim::{Anim, AnimKind, PoseDelta};
use crate::camera::Camera;
use crate::eval::CompiledTimeline;
use crate::path_util::{path_length, sample_path};
use crate::tracker::{TrackerId, Updater};
use crate::{Color, Item, ItemId, Shape};
use kurbo::BezPath;
use kurbo::{Affine, Circle, Point, Rect, Shape as _, Vec2};

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
    /// 相机对象句柄（None = 无相机；见 [`Timeline::camera`]）。
    camera: Option<ItemId>,
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
            camera: None,
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

    /// 批量注册（manim `VGroup` 注册侧语义）：返回句柄列表，配合
    /// [`Timeline::group`] / [`Timeline::together`] 做组级动画。
    pub fn add_group(&mut self, items: Vec<Item>) -> Vec<ItemId> {
        let n = items.len();
        let ids = items.into_iter().map(|it| self.add(it)).collect();
        log::debug!("add_group: {n} 项注册");
        ids
    }

    /// 保存对象当前构建期基准状态（manim `save_state` 对应；快照可多次保存）。
    ///
    /// ffanim 的动画不改注册基准，故"当前状态"即注册基准快照。
    pub fn save_state(&self, id: ItemId) -> Item {
        self.items[id as usize].clone()
    }

    /// 恢复对象到快照状态（manim `restore` 对应）：完整 morph（形状/颜色/变换）。
    ///
    /// 返回恢复后的新句柄——morph 完成后原句柄对象退场（Transform 语义），
    /// 后续动画应使用返回句柄。
    pub fn restore(&mut self, id: ItemId, snapshot: &Item) -> ItemId {
        let snap_id = self.add(snapshot.clone());
        // 埋点：恢复编排（时序排查用）
        log::info!(
            "restore: item#{id} → 快照 item#{snap_id}（morph 段 [{:.2},{:.2})）",
            self.cursor_s,
            self.cursor_s + 1.0
        );
        self.play(Anim::transform(id, snap_id));
        snap_id
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

    /// 注册相机（不可见对象，docs/02 §3.4"相机即 Item"）：相机的 transform 即 world→screen 仿射。
    ///
    /// 返回句柄；用 [`Anim::transform_to`] 把相机变换到 [`Camera::affine`] 目标即可推拉摇移。
    /// 不注册相机时求值按单位阵处理（世界系 = 屏幕系）。
    pub fn camera(&mut self, cam: Camera) -> ItemId {
        let item = Item::from_path(BezPath::new())
            .with_transform(cam.affine(self._canvas.width, self._canvas.height));
        let id = self.add(item);
        self.camera = Some(id);
        id
    }

    /// 在当前游标处排入一个动画，游标推进其时长。
    pub fn play(&mut self, mut anim: Anim) -> &mut Self {
        let overlap = self.pending_overlap_s.take().unwrap_or(0.0);
        self.prepare(&mut anim);
        let start_s = (self.cursor_s - overlap).max(0.0);
        // 埋点：沿路径移动的时序窗口（配合 prepare 的几何埋点，排错用）
        if let AnimKind::MoveAlongPath { id, .. } = &anim.kind {
            log::info!(
                "move_along_path 时序: item#{id} 窗口=[{start_s:.2},{:.2})",
                start_s + anim.duration_s
            );
        }
        self.anims.push((start_s, anim.clone()));
        self.cursor_s = start_s + anim.duration_s;
        self
    }

    /// 组动画：子动画按固定步长错开排入（manim `LaggedStart` 手感的直接形式）。
    ///
    /// 第 i 段起始 = 首段起始 + i×`step`，游标推进到最后一段结束；首段继承
    /// pending overlap（[`Timeline::overlap`]）。`step` 小于段时长时相邻段自然交叠，
    /// 等价于逐段 `play + overlap` 手工对齐（M3 踩坑点），但意图直白不易错。
    pub fn group(&mut self, anims: impl IntoIterator<Item = Anim>, step: f64) -> &mut Self {
        self.stagger(anims, |_| step)
    }

    /// [`Timeline::group`] 的 lag ratio 形式：相邻段起始间隔 = ratio × 前一段时长
    /// （manim `lagged_start` 语义；ratio=1 即顺序不交叠，ratio=0 即同时开始）。
    pub fn lagged_start(&mut self, anims: impl IntoIterator<Item = Anim>, ratio: f64) -> &mut Self {
        self.stagger(anims, |dur| ratio * dur)
    }

    /// 同窗组（manim `Together` 对应）：全部动画对齐当前游标同时开始，
    /// 游标推进到最长一段结束；首段继承 pending overlap。
    pub fn together(&mut self, anims: impl IntoIterator<Item = Anim>) -> &mut Self {
        let overlap = self.pending_overlap_s.take().unwrap_or(0.0);
        let start = (self.cursor_s - overlap).max(0.0);
        let mut end = start;
        let mut n = 0;
        for mut anim in anims {
            self.prepare(&mut anim);
            self.anims.push((start, anim.clone()));
            end = end.max(start + anim.duration_s);
            n += 1;
        }
        if n > 0 {
            self.cursor_s = end;
            // 埋点：同窗编排（动画时序排查用）
            log::info!("together: {n} 段同窗 [{start:.2},{end:.2})");
        }
        self
    }

    /// 接续组（manim `Succession` 对应）：依次排入（游标推进总和），
    /// 首段继承 pending overlap。
    pub fn succession(&mut self, anims: impl IntoIterator<Item = Anim>) -> &mut Self {
        let pending = self.pending_overlap_s.unwrap_or(0.0);
        let begin = (self.cursor_s - pending).max(0.0);
        let mut n = 0;
        for anim in anims {
            self.play(anim);
            n += 1;
        }
        if n > 0 {
            log::info!("succession: {n} 段接续 [{begin:.2},{:.2})", self.cursor_s);
        }
        self
    }

    /// 组排布内部实现：第 i 段起始 = 首段起始 + Σ step(段时长)，游标 = 最后一段结束。
    fn stagger(
        &mut self,
        anims: impl IntoIterator<Item = Anim>,
        step_of: impl Fn(f64) -> f64,
    ) -> &mut Self {
        let overlap = self.pending_overlap_s.take().unwrap_or(0.0);
        let mut start = (self.cursor_s - overlap).max(0.0);
        let mut end = self.cursor_s;
        let mut n = 0;
        for mut anim in anims {
            self.prepare(&mut anim);
            self.anims.push((start, anim.clone()));
            end = start + anim.duration_s;
            start += step_of(anim.duration_s).max(0.0);
            n += 1;
        }
        if n > 0 {
            self.cursor_s = end;
        }
        self
    }

    /// 构建期推演（play/group 共用）：SetValue 与 TransformTo 的 from 取构建期当前值，
    /// 对象/tracker 基线快进到目标（求值期段前取 from 的语义见 `eval::transform_at`）；
    /// Pose（相对位姿）解析为绝对 TransformTo。
    fn prepare(&mut self, anim: &mut Anim) {
        if let AnimKind::SetValue { tracker, from, to } = &mut anim.kind {
            *from = self.tracker_cur[*tracker];
            self.tracker_cur[*tracker] = *to;
        }
        if let AnimKind::TransformTo { id, from, to } = &mut anim.kind {
            let idx = *id as usize;
            assert!(idx < self.items.len(), "TransformTo 目标对象 #{id} 未注册");
            let target = *to;
            *from = self.items[idx].transform;
            self.items[idx].transform = target;
        }
        if let AnimKind::Pose { id, delta } = anim.kind.clone() {
            // 相对位姿：由基线与 bbox 中心算绝对目标后按 TransformTo 落表
            let idx = id as usize;
            assert!(idx < self.items.len(), "Pose 目标对象 #{id} 未注册");
            let from = self.items[idx].transform;
            let center = self.items[idx].bbox().center();
            let to = delta.resolve(from, center);
            // 埋点：位姿解析全链路（坐标计算排查用）
            log::info!(
                "位姿解析: item#{id} delta={delta:?} 基线={from:?} 中心=({:.1},{:.1}) → 目标={to:?}",
                center.x,
                center.y
            );
            self.items[idx].transform = to;
            anim.kind = AnimKind::TransformTo { id, from, to };
        }
        if let AnimKind::MoveAlongPath { id, path, base } = &mut anim.kind {
            // 沿路径移动：base 取构建期基线，对象基线快进到路径终点
            //（段后驻停终点 + 后续动画基于终点接续，与 TransformTo 快进语义一致）
            let idx = *id as usize;
            assert!(
                idx < self.items.len(),
                "MoveAlongPath 目标对象 #{id} 未注册"
            );
            *base = self.items[idx].transform;
            // 起点对齐：位移 = 采样点 − 路径起点（与对象初始位置无关）
            let start = sample_path(path, 0.0);
            let end = sample_path(path, 1.0);
            let end_off = Vec2::new(end.x - start.x, end.y - start.y);
            let advanced = Affine::translate(end_off) * *base;
            // 埋点：轨迹几何全链路（弧长/路径范围/基线推进量，坐标计算排查用）
            log::info!(
                "move_along_path 解析: item#{id} 轨迹 {:.1}pt（{:?}）起点=({:.1},{:.1}) 终点=({:.1},{:.1}) 终点偏移=({:.1},{:.1}) 基线={:?} → 推进后={advanced:?} dur={:.2}s",
                path_length(path),
                path.bounding_box(),
                start.x,
                start.y,
                end.x,
                end.y,
                end_off.x,
                end_off.y,
                *base,
                anim.duration_s
            );
            self.items[idx].transform = advanced;
        }
    }

    // ---- 强调动画（manim Indicate/Circumscribe/Flash/ShowPassingFlash 对应）----
    // 全部为构建期编排：展开为既有词汇（TransformTo / FadeIn / FadeOut），
    // 求值路径零改动；核心分支 log::info 埋点（时序与坐标排查用）。

    /// 闪现强调（manim `Indicate`）：绕对象 bbox 中心放大 `factor` 后回到原位。
    ///
    /// 展开为两段 TransformTo（前半程放大、后半程复原），段后基线复位原位；
    /// 游标推进 `dur`。
    pub fn indicate(&mut self, id: ItemId, factor: f64, dur: f64) -> &mut Self {
        let idx = id as usize;
        assert!(idx < self.items.len(), "indicate 目标对象 #{id} 未注册");
        let from = self.items[idx].transform;
        let center = self.items[idx].bbox().center();
        let to = PoseDelta::Scale(factor).resolve(from, center);
        let t0 = self.cursor_s;
        // 埋点：峰值变换的坐标计算全链路
        log::info!(
            "indicate: item#{id} factor={factor} 中心=({:.1},{:.1}) 基线={from:?} → 峰值={to:?}",
            center.x,
            center.y
        );
        self.play(Anim::scale_to(id, factor).during(dur / 2.0));
        self.play(Anim::transform_to(id, from).during(dur / 2.0));
        // 埋点：两段时序窗口
        log::info!(
            "indicate 时序: item#{id} 放大段=[{t0:.2},{:.2}) 复原段=[{:.2},{:.2})",
            t0 + dur / 2.0,
            t0 + dur / 2.0,
            t0 + dur
        );
        self
    }

    /// 框选强调（manim `Circumscribe`）：围绕对象包围盒外扩 `buff` 的矩形框
    /// 淡入后淡出（前半程淡入、后半程淡出）。
    ///
    /// 框为临时对象（fade_in 段前不可见、fade_out 段后隐藏），游标推进 `dur`。
    pub fn circumscribe(&mut self, id: ItemId, dur: f64, buff: f64, color: Color) -> &mut Self {
        let target = self.items[id as usize].bbox();
        let rect = Rect::new(
            target.x0 - buff,
            target.y0 - buff,
            target.x1 + buff,
            target.y1 + buff,
        );
        let frame = Item::rect(rect).with_stroke(color).with_stroke_width(3.0);
        let fid = self.add(frame);
        let t0 = self.cursor_s;
        self.play(Anim::fade_in(fid).during(dur / 2.0));
        self.play(Anim::fade_out(fid).during(dur / 2.0));
        // 埋点：框范围（坐标计算）+ 淡入/淡出窗口（时序）
        log::info!(
            "circumscribe: item#{id} 目标bbox={target:?} buff={buff} 框={rect:?}；时序 fade_in=[{t0:.2},{:.2}) fade_out=[{:.2},{:.2})",
            t0 + dur / 2.0,
            t0 + dur / 2.0,
            t0 + dur
        );
        self
    }

    /// 闪现（manim `Flash`）：在 `at` 处闪一个扩张淡出的圆环——半径 `radius`
    /// 的圆环全程平滑放大 2 倍，前半程淡入、后半程淡出；游标推进 `dur`。
    pub fn flash(
        &mut self,
        at: impl Into<Point>,
        dur: f64,
        radius: f64,
        color: Color,
    ) -> &mut Self {
        let at = at.into();
        assert!(radius > 0.0 && dur > 0.0, "flash 半径与时长须为正");
        let ring = Item::circle(Circle::new(at, radius))
            .with_stroke(color)
            .with_stroke_width(3.0);
        let fid = self.add(ring);
        let t0 = self.cursor_s;
        // 同窗编排：fade_in 前半 + 放大全程（overlap 回退对齐）+ fade_out 后半
        self.play(Anim::fade_in(fid).during(dur / 2.0));
        self.overlap(dur / 2.0);
        self.play(Anim::scale_to(fid, 2.0).during(dur));
        self.overlap(dur / 2.0);
        self.play(Anim::fade_out(fid).during(dur / 2.0));
        // 埋点：中心/半径（坐标计算）+ 三段窗口（时序）
        log::info!(
            "flash: 中心=({:.1},{:.1}) radius={radius} 扩张 1→2 倍；时序 fade_in=[{t0:.2},{:.2}) 放大=[{t0:.2},{:.2}) fade_out=[{:.2},{:.2})",
            at.x,
            at.y,
            t0 + dur / 2.0,
            t0 + dur,
            t0 + dur / 2.0,
            t0 + dur
        );
        self
    }

    /// 路径经过闪烁（manim `ShowPassingFlash`）：沿对象路径的 `[t0, t1]` 比例段
    /// 临时显示一条高亮描边（前半程淡入、后半程淡出），原对象不动；游标推进 `dur`。
    ///
    /// 段提取为段粒度（[`crate::path_util::span_path`]），高亮描边继承原对象的变换。
    pub fn show_passing_flash(
        &mut self,
        id: ItemId,
        t0: f64,
        t1: f64,
        dur: f64,
        color: Color,
    ) -> &mut Self {
        let base = &self.items[id as usize];
        let transform = base.transform;
        let Shape::Path(path) = &base.shape else {
            panic!("show_passing_flash 目标须为路径对象（item#{id}）");
        };
        let seg = crate::path_util::span_path(path, t0, t1);
        let elems = seg.elements().len();
        let item = Item::from_path(seg)
            .with_stroke(color)
            .with_stroke_width(3.0)
            .with_transform(transform);
        let sid = self.add(item);
        let start = self.cursor_s;
        self.play(Anim::fade_in(sid).during(dur / 2.0));
        self.play(Anim::fade_out(sid).during(dur / 2.0));
        // 埋点：路径段比例与提取结果（坐标计算）+ 淡入/淡出窗口（时序）
        log::info!(
            "show_passing_flash: item#{id} 路径段=[{t0},{t1}] → {elems} 元素（继承原变换）；时序 fade_in=[{start:.2},{:.2}) fade_out=[{:.2},{:.2})",
            start + dur / 2.0,
            start + dur / 2.0,
            start + dur
        );
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
            camera: self.camera,
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

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Affine, Point};

    const BG: crate::Color = crate::Color::new(14, 17, 22, 255);

    /// 100x100 矩形（原点在 (0,0)），compile 后返回（anims/items 语义断言用）。
    fn rect_timeline() -> (Timeline, ItemId) {
        let mut tl = Timeline::new(960, 540, 30, BG);
        let id = tl.add(Item::rect(kurbo::Rect::new(0.0, 0.0, 100.0, 100.0)));
        (tl, id)
    }

    fn affine_close(a: Affine, b: Affine) -> bool {
        a.as_coeffs()
            .iter()
            .zip(b.as_coeffs())
            .all(|(x, y)| (x - y).abs() < 1e-9)
    }

    /// 100x100 @30fps 最小时间轴。
    fn make_tl() -> Timeline {
        Timeline::new(100, 100, 30, Color::new(0, 0, 0, 255))
    }

    /// 40x40 填充矩形（原点 (0,0)，中心 (20,20)，断言坐标计算用）。
    fn rect_item(fill: crate::Color) -> Item {
        Item::rect(kurbo::Rect::new(0.0, 0.0, 40.0, 40.0)).with_fill(fill)
    }

    #[test]
    fn 位姿_平移解析为绝对变换并快进基线() {
        let (mut tl, id) = rect_timeline();
        tl.play(Anim::shift_to(id, 50.0, 0.0).during(1.0));
        let c = tl.compile();
        // 落表为 TransformTo：from = 单位阵，to = 平移(50,0)
        assert_eq!(c.anims.len(), 1);
        assert!(
            matches!(&c.anims[0].kind, AnimKind::TransformTo { from, to, .. }
                if *from == Affine::IDENTITY && affine_close(*to, Affine::translate((50.0, 0.0)))),
            "应为绝对 TransformTo: {:?}",
            c.anims[0].kind
        );
        // 基线快进：对象变换 = 目标位姿
        assert!(affine_close(
            c.items[0].transform,
            Affine::translate((50.0, 0.0))
        ));
    }

    #[test]
    fn 位姿_连续动作基于前段基线() {
        let (mut tl, id) = rect_timeline();
        tl.play(Anim::shift_to(id, 50.0, 0.0).during(1.0));
        tl.play(Anim::scale_to(id, 2.0).during(1.0));
        let c = tl.compile();
        let AnimKind::TransformTo { from, to, .. } = &c.anims[1].kind else {
            panic!("第二段应为 TransformTo");
        };
        // from = 第一段终点；中心 = 平移后 bbox 中心 (100, 50)
        assert!(affine_close(*from, Affine::translate((50.0, 0.0))));
        let expect = Affine::translate((100.0, 50.0))
            * Affine::scale(2.0)
            * Affine::translate((-100.0, -50.0))
            * Affine::translate((50.0, 0.0));
        assert!(affine_close(*to, expect), "to = 绕新中心缩放: {to:?}");
    }

    #[test]
    fn 位姿_旋转视觉逆时针() {
        let (mut tl, id) = rect_timeline();
        tl.play(Anim::rotate_to(id, std::f64::consts::FRAC_PI_2).during(1.0));
        let c = tl.compile();
        let AnimKind::TransformTo { to, .. } = &c.anims[0].kind else {
            panic!("应为 TransformTo");
        };
        // bbox 中心 (50,50)；右侧点 (100,50) 视觉逆时针 → 顶点 (50,0)
        let p = *to * Point::new(100.0, 50.0);
        assert!(
            (p.x - 50.0).abs() < 1e-9 && (p.y - 0.0).abs() < 1e-9,
            "右点应到顶点: {p:?}"
        );
    }

    #[test]
    fn 位姿_移动到目标点对齐中心() {
        let (mut tl, id) = rect_timeline();
        tl.play(Anim::move_to(id, (300.0, 200.0)).during(1.0));
        let c = tl.compile();
        let AnimKind::TransformTo { to, .. } = &c.anims[0].kind else {
            panic!("应为 TransformTo");
        };
        // 原 bbox 中心 (50,50) → 平移 (250,150)
        assert!(
            affine_close(*to, Affine::translate((250.0, 150.0))),
            "{to:?}"
        );
    }

    #[test]
    fn 位姿_未注册对象报错() {
        let mut tl = Timeline::new(960, 540, 30, BG);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tl.play(Anim::shift_to(42, 1.0, 1.0));
        }));
        assert!(result.is_err(), "未注册对象应 panic");
    }

    #[test]
    fn 强调_闪现强调两段往返复位() {
        use crate::eval::render_frame;
        let (mut tl, id) = rect_timeline();
        tl.play(Anim::create(id).during(1.0));
        tl.indicate(id, 1.2, 1.0); // 游标 = 2.0s
        let c = tl.compile();
        assert_eq!(c.anims.len(), 3);
        // 峰值段 to：绕中心 (50,50) 放大 1.2；复原段 to = 原位（单位阵）
        let AnimKind::TransformTo { to: peak, .. } = &c.anims[1].kind else {
            panic!("第二段应为峰值 TransformTo");
        };
        let expect = Affine::translate((50.0, 50.0))
            * Affine::scale(1.2)
            * Affine::translate((-50.0, -50.0));
        assert!(affine_close(*peak, expect), "{peak:?}");
        let AnimKind::TransformTo { to, .. } = &c.anims[2].kind else {
            panic!("第三段应为复原 TransformTo");
        };
        assert!(affine_close(*to, Affine::IDENTITY), "{to:?}");
        // 时序窗口：放大 [30,45) / 复原 [45,60)，总 60 帧
        assert_eq!((c.anims[1].start, c.anims[1].end), (30, 45));
        assert_eq!((c.anims[2].start, c.anims[2].end), (45, 60));
        assert_eq!(c.duration_frames, 60);
        // 坐标语义：复原段首帧（45）= 峰值；段后（60）基线复位原位
        assert!(
            affine_close(render_frame(&c, 45).items[0].transform, expect),
            "复原段首帧应为峰值"
        );
        assert!(
            affine_close(render_frame(&c, 60).items[0].transform, Affine::IDENTITY),
            "段后应复位原位"
        );
    }

    #[test]
    fn 强调_框选强调时序与框范围() {
        use crate::eval::render_frame;
        let (mut tl, id) = rect_timeline(); // rect (0,0,100,100)
        tl.play(Anim::create(id).during(1.0));
        tl.circumscribe(id, 0.8, 20.0, Color::new(255, 0, 0, 255));
        let c = tl.compile();
        // 框对象 = 第 2 个 item：目标 bbox 外扩 20
        let bb = c.items[1].bbox();
        assert!(
            (bb.x0 + 20.0).abs() < 1e-9 && (bb.x1 - 120.0).abs() < 1e-9,
            "{bb:?}"
        );
        // 两段窗口 [30,42) / [42,54)，游标 1.8s = 54 帧
        assert_eq!((c.anims[1].start, c.anims[1].end), (30, 42));
        assert_eq!((c.anims[2].start, c.anims[2].end), (42, 54));
        assert_eq!(c.duration_frames, 54);
        // 可见性：帧 0 create 进度 0 → 全空 / 中段框可见 / 结束后框隐藏
        assert_eq!(
            render_frame(&c, 0).items.len(),
            0,
            "帧 0 create 进度 0 → 空"
        );
        assert_eq!(render_frame(&c, 35).items.len(), 2, "框淡入中可见");
        assert_eq!(render_frame(&c, 54).items.len(), 1, "框结束后隐藏");
    }

    #[test]
    fn 强调_闪现三段同窗() {
        use crate::eval::render_frame;
        let mut tl = Timeline::new(200, 200, 30, BG);
        tl.flash((100.0, 100.0), 0.6, 20.0, Color::new(255, 200, 0, 255));
        let c = tl.compile();
        // fade_in [0,9) / 放大 [0,18) / fade_out [9,18)
        let windows: Vec<(usize, usize)> = c.anims.iter().map(|a| (a.start, a.end)).collect();
        assert_eq!(windows, vec![(0, 9), (0, 18), (9, 18)]);
        assert_eq!(c.duration_frames, 18);
        // 放大段 to：绕圆心 (100,100) 放大 2 倍（圆 bbox 中心 = 圆心）
        let AnimKind::TransformTo { to, .. } = &c.anims[1].kind else {
            panic!("放大段应为 TransformTo");
        };
        let expect = Affine::translate((100.0, 100.0))
            * Affine::scale(2.0)
            * Affine::translate((-100.0, -100.0));
        assert!(affine_close(*to, expect), "{to:?}");
        // 可见性：帧 0 透明度为 0 → 空；闪现中可见；结束后隐藏
        assert_eq!(render_frame(&c, 0).items.len(), 0);
        assert_eq!(render_frame(&c, 5).items.len(), 1);
        assert_eq!(render_frame(&c, 18).items.len(), 0);
    }

    #[test]
    fn 强调_路径经过闪烁窗口() {
        use crate::eval::render_frame;
        let (mut tl, id) = rect_timeline();
        tl.play(Anim::create(id).during(1.0));
        tl.show_passing_flash(id, 0.0, 0.5, 0.8, Color::new(255, 255, 0, 255));
        let c = tl.compile();
        // 高亮段对象 = 第 2 个 item：矩形路径 4 段取前 2 段（move + 2 line）
        let Shape::Path(p) = &c.items[1].shape else {
            panic!("高亮对象应为路径");
        };
        assert_eq!(p.elements().len(), 3, "{:?}", p.elements());
        // 两段窗口 [30,42) / [42,54)
        assert_eq!((c.anims[1].start, c.anims[1].end), (30, 42));
        assert_eq!((c.anims[2].start, c.anims[2].end), (42, 54));
        assert_eq!(c.duration_frames, 54);
        // 高亮段帧 35 可见、段后隐藏
        assert_eq!(render_frame(&c, 35).items.len(), 2);
        assert_eq!(render_frame(&c, 54).items.len(), 1);
    }

    // ---- M9.7：P1 薄糖批（Together / Succession / add_group / save_state-restore）----

    #[test]
    fn together_同窗对齐游标取最长() {
        let mut tl = make_tl();
        let a = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        let b = tl.add(rect_item(Color::new(0, 255, 0, 255)));
        tl.wait(0.5);
        tl.together([
            Anim::create(a).during(1.0),
            Anim::fade_in(b).during(2.0), // 时长不同：游标应取最长
        ]);
        let c = tl.compile();
        let windows: Vec<(usize, usize)> = c.anims.iter().map(|x| (x.start, x.end)).collect();
        assert_eq!(windows, vec![(15, 45), (15, 75)], "两段同窗对齐 0.5s");
        assert_eq!(c.duration_frames, 75, "游标 = 最长段结束");
    }

    #[test]
    fn succession_接续游标累加() {
        let mut tl = make_tl();
        let a = tl.add(rect_item(Color::new(255, 0, 0, 255)));
        let b = tl.add(rect_item(Color::new(0, 255, 0, 255)));
        tl.succession([Anim::fade_in(a).during(0.5), Anim::fade_in(b).during(1.0)]);
        let c = tl.compile();
        let windows: Vec<(usize, usize)> = c.anims.iter().map(|x| (x.start, x.end)).collect();
        assert_eq!(windows, vec![(0, 15), (15, 45)], "依次接续不交叠");
        assert_eq!(c.duration_frames, 45);
    }

    #[test]
    fn add_group_批量注册() {
        let mut tl = make_tl();
        let ids = tl.add_group(vec![
            rect_item(Color::new(255, 0, 0, 255)),
            rect_item(Color::new(0, 255, 0, 255)),
            rect_item(Color::new(0, 0, 255, 255)),
        ]);
        assert_eq!(ids, vec![0, 1, 2]);
        tl.group(ids.iter().map(|&id| Anim::fade_in(id).during(0.5)), 0.1);
        let c = tl.compile();
        assert_eq!(c.items.len(), 3);
        assert_eq!(c.anims.len(), 3);
    }

    #[test]
    fn save_state_restore_完整恢复() {
        use crate::eval::render_frame;
        let mut tl = make_tl();
        let id = tl.add(rect_item(Color::new(255, 0, 0, 255))); // 基线 = 单位阵
        let snap = tl.save_state(id);
        tl.play(Anim::move_to(id, (80.0, 40.0)).during(1.0)); // 基线快进到 (80,40)
        let restored = tl.restore(id, &snap); // morph 回快照（默认 1.0s）
        let c = tl.compile();

        // morph 段 [30,60)：段后旧对象退场、快照对象在场且复位
        let after = render_frame(&c, 60);
        assert_eq!(after.items.len(), 1, "morph 后只余快照对象");
        assert_eq!(after.items[0].id, restored);
        assert_eq!(
            after.items[0].transform,
            Affine::IDENTITY,
            "应复位到快照基线"
        );
        assert_eq!(c.duration_frames, 60);
    }
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
