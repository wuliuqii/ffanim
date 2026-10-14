//! 花括号（manim `Brace` 对应，P1-8）：Typst MATH 装配机制。
//!
//! 形状来源 = NewCMMath 字体的 OpenType MATH 表字形装配（GlyphAssembly）：
//! 顶钩 + 可重复延长臂段 + 中尖角 + 底钩，按 `lr(size:)` 目标高度堆叠、
//! 片间按 MATH 表 `MinConnectorOverlap` 重叠防接缝。与 manim 的"CM 字形 +
//! 线性段拉伸"同源同思想，但拼接质量由字体表保证，且跨度/深度完全解耦：
//! - 跨度：`lr(size: #Npt, \{)` 精确到像素（装配延长臂吸收）；
//! - 深度：字号线性决定（深度 ∝ 字号，钩/尖角比例保真，比例运行时探针实测）。
//!
//! 单位约定：排版内部为 pt（y 向下），归一化仿射统一换算到画布 px
//! （1pt = 96/72 px），产物路径 x ∈ [0, span]、y ∈ [0, depth]、开口线 y=0，
//! 与原 core 模板实现的空间语义一致。样式由调用方链式 `with_fill` 设置。

use ffanim_core::kurbo::{Affine, BezPath, Point, Rect, Shape as _};
use ffanim_core::text::Font;
use ffanim_core::{Color, Direction, Error, Item, Result};

use crate::engine::TypstEngine;
use crate::formula::FormulaOptions;

/// pt → px（Typst 排版 pt 空间 → ffanim 画布 px 空间）。
const PT_TO_PX: f64 = 96.0 / 72.0;
/// px → pt（Typst 无 `px` 单位，跨度以 pt 表达）。
const PX_TO_PT: f64 = 72.0 / 96.0;
/// 深度探针体：100pt 字号排 750pt 跨度装配，实测"深度/字号"比（字体常量）。
const PROBE_BODY: &str = "lr(size: #750pt, \\{)";
/// 探针字号（pt）。
const PROBE_FONT_PT: f64 = 100.0;

impl TypstEngine {
    /// 花括号（manim `Brace`）：沿目标包围盒 `dir` 侧、间隔 `buff` 生成，
    /// 开口朝向目标、中尖角指向外侧。填充字形，样式建议 `with_fill` 着色。
    ///
    /// 跨度 = 目标 bbox 对应边长；深度随跨度自适应（钳制防过粗/过细）。
    /// 出场动画用 `fade_in`（manim `GrowFromCenter` 对应）而非 `create`
    /// （填充字形描边生长会显式描出轮廓过程）。
    ///
    /// # Errors
    /// 跨度过短（< 30px）或 Typst 编译失败时返回错误。
    pub fn brace(&mut self, target: &Item, dir: Direction, buff: f64) -> Result<Item> {
        let b = target.bbox();
        // 规范形：跨度沿 +x（原点 = 逆时针首端），开口朝 -y（对象在上），中尖角 +y
        let (span, anchor, rot) = match dir {
            // 下方：开口朝上（-y），跨度左 → 右
            Direction::Down => (b.x1 - b.x0, Point::new(b.x0, b.y1 + buff), Affine::IDENTITY),
            // 上方：旋转 π，跨度右 → 左（等价左右镜像），开口朝下
            Direction::Up => (
                b.x1 - b.x0,
                Point::new(b.x1, b.y0 - buff),
                Affine::rotate(std::f64::consts::PI),
            ),
            // 左侧：旋转 +90°，跨度上 → 下，开口朝右（+x）
            Direction::Left => (
                b.y1 - b.y0,
                Point::new(b.x0 - buff, b.y0),
                Affine::rotate(std::f64::consts::FRAC_PI_2),
            ),
            // 右侧：旋转 -90°，跨度下 → 上，开口朝左（-x）
            Direction::Right => (
                b.y1 - b.y0,
                Point::new(b.x1 + buff, b.y1),
                Affine::rotate(-std::f64::consts::FRAC_PI_2),
            ),
        };
        assert!(span >= 30.0, "brace 跨度过短: {span:.1}px（目标侧边太窄）");
        let k = (span * 0.09).clamp(8.0, 18.0);
        // 深度系数 1.2：540p 画布下宽跨度 ≈ 21.6px，观感与 manim 等效深度一致
        let depth = 1.2 * k;
        let path = self.brace_path(span, depth)?;
        let item =
            Item::from_path(path).with_transform(Affine::translate((anchor.x, anchor.y)) * rot);
        log::info!(
            "brace: dir={dir:?} 目标 bbox={b:?} buff={buff} → 跨度 {span:.1} 深 {depth:.1} 结果 bbox={:?}",
            item.bbox()
        );
        Ok(item)
    }

    /// 花括号 + 括注文本（manim `brace.get_text` 对应）：返回 (brace, 文本) 散件对，
    /// 文本置于中尖角外侧居中（间距 = 0.45 × 字号）。样式由调用方链式设置。
    ///
    /// # Errors
    /// 花括号排版失败或文本字形提取失败时返回错误。
    pub fn brace_with_text(
        &mut self,
        target: &Item,
        dir: Direction,
        buff: f64,
        font: &Font,
        label: &str,
        size: f64,
    ) -> Result<(Item, Item)> {
        assert!(!label.is_empty(), "brace 括注文本不能为空");
        let brace = self.brace(target, dir, buff)?;
        let bb = brace.bbox();
        let text_path = font.outline_text(label, size)?;
        let tb = text_path.bounding_box();
        let gap = 0.45 * size;
        // 文本中心对齐 brace bbox 中心（沿跨度方向），外沿留 gap
        let (dx, dy) = match dir {
            Direction::Down => (
                (bb.x0 + bb.x1) / 2.0 - (tb.x0 + tb.x1) / 2.0,
                bb.y1 + gap - tb.y0,
            ),
            Direction::Up => (
                (bb.x0 + bb.x1) / 2.0 - (tb.x0 + tb.x1) / 2.0,
                bb.y0 - gap - tb.y1,
            ),
            Direction::Left => (
                bb.x0 - gap - tb.x1,
                (bb.y0 + bb.y1) / 2.0 - (tb.y0 + tb.y1) / 2.0,
            ),
            Direction::Right => (
                bb.x1 + gap - tb.x0,
                (bb.y0 + bb.y1) / 2.0 - (tb.y0 + tb.y1) / 2.0,
            ),
        };
        let text = Item::from_path(text_path).with_transform(Affine::translate((dx, dy)));
        log::info!(
            "brace_with_text: label={label:?} size={size} dir={dir:?} → brace bbox={bb:?} 文本 bbox={:?}",
            text.bbox()
        );
        Ok((brace, text))
    }

    /// 装配花括号路径：跨度/深度（画布 px）→ 归一化单路径轮廓
    /// （x ∈ [0, span]，y ∈ [0, depth]，开口线 y=0，多子路径同色填充）。
    fn brace_path(&mut self, span_px: f64, depth_px: f64) -> Result<BezPath> {
        // ① 深度探针：100pt 字号排 750pt 跨度装配，实测深度/字号比（LRU 缓存，一次成本）
        let probe = self.formula(
            PROBE_BODY,
            &FormulaOptions {
                font_size_pt: PROBE_FONT_PT,
                text_color: Color::new(255, 255, 255, 255),
            },
        )?;
        let pb = formula_outline_bbox(&probe);
        // 每 1pt 字号对应的装配深度（px）——NewCMMath 花括号装配常量 ≈ 0.80
        let depth_per_font_px = pb.width() * PT_TO_PX / PROBE_FONT_PT;
        if depth_per_font_px <= 0.0 {
            return Err(Error::Other("brace 探针异常：装配深度非正".into()));
        }
        // ② 按目标跨度排版（px→pt）：lr(size:) 由装配延长臂精确吸收（实测误差 < 0.5px）
        let body = format!("lr(size: #{}pt, \\{{)", span_px * PX_TO_PT);
        let f = self.formula(
            &body,
            &FormulaOptions {
                font_size_pt: depth_px / depth_per_font_px,
                text_color: Color::new(255, 255, 255, 255),
            },
        )?;
        // ③ 各装配片合并为单路径（同色不透明填充下重叠区无渲染瑕疵）
        let mut path = BezPath::new();
        for p in &f.pieces {
            path.extend(p.path.elements().iter().copied());
        }
        // ④ 归一化：pt→px 缩放 + 旋转 -90°（开口 +x → -y）+ 平移到 [0,span]×[0,depth]
        //    （旋转把开口线（原 x 最大）转到 y 最小，中尖角（原 x 最小）转到 y 最大）
        let bb = path.bounding_box();
        if !(bb.width() > 0.0 && bb.height() > 0.0) {
            return Err(Error::Other("brace 装配路径为空".into()));
        }
        path.apply_affine(
            Affine::translate((-PT_TO_PX * bb.y0, PT_TO_PX * bb.x1))
                * Affine::rotate(-std::f64::consts::FRAC_PI_2)
                * Affine::scale(PT_TO_PX),
        );
        Ok(path)
    }
}

/// 公式全部片段路径的联合包围盒（pt，控制点界）。
fn formula_outline_bbox(f: &crate::Formula) -> Rect {
    f.pieces
        .iter()
        .map(|p| p.path.bounding_box())
        .reduce(|a, b| a.union(b))
        .unwrap_or(Rect::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffanim_core::kurbo::Rect;

    #[test]
    fn brace_下方几何与跨度自适应() {
        let mut engine = TypstEngine::new(8);
        let target = Item::rect(Rect::new(100.0, 100.0, 300.0, 150.0));
        let brace = engine.brace(&target, Direction::Down, 12.0).unwrap();
        let bb = brace.bbox();
        // 跨度 = 目标宽 200（装配 size 参数精确）；深度 = 1.2×clamp(200×0.09=18) = 21.6
        assert!(
            (bb.x0 - 100.0).abs() < 0.5 && (bb.x1 - 300.0).abs() < 0.5,
            "跨度 {bb:?}"
        );
        assert!((bb.y0 - 162.0).abs() < 0.5, "开口线贴 buff {bb:?}");
        assert!((bb.y1 - 183.6).abs() < 1.0, "深度尖角 {bb:?}");
    }

    #[test]
    fn brace_四方向外沿定位() {
        let mut engine = TypstEngine::new(8);
        let target = Item::rect(Rect::new(100.0, 100.0, 300.0, 200.0));
        // 外沿 = buff 线（装配归一化精确，容差只为浮点误差）
        let up = engine.brace(&target, Direction::Up, 10.0).unwrap().bbox();
        assert!((up.y1 - 90.0).abs() < 0.5, "上方 brace 外沿 y1={}", up.y1);
        let left = engine.brace(&target, Direction::Left, 10.0).unwrap().bbox();
        assert!(
            (left.x1 - 90.0).abs() < 0.5,
            "左侧 brace 外沿 x1={}",
            left.x1
        );
        let right = engine
            .brace(&target, Direction::Right, 10.0)
            .unwrap()
            .bbox();
        assert!(
            (right.x0 - 310.0).abs() < 0.5,
            "右侧 brace 外沿 x0={}",
            right.x0
        );
        // 沿跨度方向与目标居中对齐
        let down = engine.brace(&target, Direction::Down, 10.0).unwrap().bbox();
        assert!((down.x0 + down.x1 - 400.0).abs() < 0.5, "{down:?}");
    }

    #[test]
    fn brace_括注文本外沿居中() {
        let Some(path) = crate::text::find_cjk_font() else {
            eprintln!("无 CJK 字体，跳过");
            return;
        };
        let mut engine = TypstEngine::new(8);
        let font = Font::from_path(&path).unwrap();
        let target = Item::rect(Rect::new(100.0, 100.0, 300.0, 150.0));
        let (brace, label) = engine
            .brace_with_text(&target, Direction::Down, 12.0, &font, "宽度", 28.0)
            .unwrap();
        let bb = brace.bbox();
        let lb = label.bbox();
        assert!(lb.y0 >= bb.y1, "文本应在尖角下方 {lb:?} vs {bb:?}");
        assert!(
            (lb.x0 + lb.x1 - bb.x0 - bb.x1).abs() < 1.0,
            "水平居中 {lb:?} vs {bb:?}"
        );
    }

    #[test]
    fn brace_缓存命中零成本() {
        let mut engine = TypstEngine::new(8);
        let target = Item::rect(Rect::new(100.0, 100.0, 300.0, 150.0));
        let a = engine.brace(&target, Direction::Down, 12.0).unwrap();
        let t0 = std::time::Instant::now();
        let b = engine.brace(&target, Direction::Down, 12.0).unwrap();
        // 二次生成走公式 LRU（探针 + 同 body），应远快于首次排版
        assert!(t0.elapsed().as_millis() < 50, "缓存后应近零成本");
        assert_eq!(a.bbox(), b.bbox());
    }
}
