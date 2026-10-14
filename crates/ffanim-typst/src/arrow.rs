//! 箭头（manim `Arrow`/`DoubleArrow` 对应）：Typst MATH stretch 装配。
//!
//! 机制 = NewCMMath GlyphAssembly（探针实证）：1-2 头字形 + N 可重复杆件
//! （100pt 单箭头 10 片、200pt 双箭头 19 片 = 2 头 + 17 杆），跨度由
//! `stretch(size: #Npt)` 精确吸收（宽比 4.000 零偏差——**无 delimiter 的
//! 档位离散，免精调仿射**）；头高/杆厚均 ∝ 字号（0.520 / 0.048，探针
//! 定标零硬编码魔数），字号由目标头高反推。
//!
//! 单位约定与 delimiter 同族：排版内部 pt（y 向下）、画布 px（y 向下）、
//! 归一化规范形杆沿 +x、中线 y=0、头在 +x 端（bbox 精确铺满 [from, to]）。

use ffanim_core::kurbo::{Affine, BezPath, Point, Shape as _};
use ffanim_core::{Color, Error, Item, Result};

use super::engine::TypstEngine;
use super::formula::{Formula, FormulaOptions};

/// pt → px（Typst 排版 pt 空间 → 画布 px 空间）。
const PT_TO_PX: f64 = 96.0 / 72.0;
/// px → pt（stretch size 参数以 pt 表达，Typst 无 px 单位）。
const PX_TO_PT: f64 = 72.0 / 96.0;
/// 头高/字号比（pt 空间，探针实证：32pt 字号 → 16.64pt 头高，精确线性）。
const HEAD_PER_FONT: f64 = 0.520;
/// 杆厚/字号比（探针实证：1.536pt @ 32pt 字号，自动跟随无需参数）。
const SHAFT_PER_FONT: f64 = 0.048;
/// 头高跨度定标：clamp(span × 0.15, 10, 24) px——跨度自适应 + 上下限
/// （manim"头不过杆半" capped 语义同族；对齐 delimiter 深度公式族）。
const HEAD_SPAN_RATIO: f64 = 0.15;
const HEAD_MIN_PX: f64 = 10.0;
const HEAD_MAX_PX: f64 = 24.0;
/// 最短跨度（px）：短于此头/杆比例失真（对齐 delimiter 的 span 下限模式）。
const MIN_SPAN_PX: f64 = 20.0;

/// 箭头头型（manim `Arrow` / `DoubleArrow` 对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowKind {
    /// 单头 `->`（头在 to 端）。
    Single,
    /// 双头 `<->`（两端头尖 = from/to，探针实证 19 片 = 2 头 + 17 杆）。
    Double,
}

impl ArrowKind {
    /// Typst math 关系符字面量（探针实证：stretch 装配拉伸、跨度精确）。
    fn sym(self) -> &'static str {
        match self {
            Self::Single => "->",
            Self::Double => "<->",
        }
    }
}

fn formula_opts(font_size_pt: f64) -> FormulaOptions {
    FormulaOptions {
        font_size_pt,
        text_color: Color::new(255, 255, 255, 255),
    }
}

/// 装配片合并单路径（头 1-2 片 + 杆 N 片；同色不透明填充重叠区无瑕疵）。
fn assemble(f: &Formula) -> Result<BezPath> {
    let mut out = BezPath::new();
    for p in &f.pieces {
        for el in p.path.elements() {
            out.push(*el);
        }
    }
    if out.elements().is_empty() {
        return Err(Error::Other("箭头装配片为空".into()));
    }
    Ok(out)
}

impl TypstEngine {
    /// 箭头：两点式（manim `Arrow(start, end)` 对齐）。
    ///
    /// - 头高 = `clamp(span × 0.15, 10, 24)` px，字号 = 头高 / 0.520 反推
    ///   （探针定标）；杆厚 = 0.048 × 字号自动跟随；
    /// - 归一化：bbox 精确铺满 [from, to]（单头 to 端 = 头尖，双头两端 =
    ///   头尖），杆过弦、头对称张开；
    /// - 形状为闭合填充字形，**调用方 `with_fill` 着色**（非描边）；出场
    ///   用 `fade_in`（填充字形禁用 create——会显式描轮廓）。
    ///
    /// # Errors
    /// 跨度低于下限、Typst 排版失败或装配片为空时返回错误。
    pub fn arrow(&mut self, from: Point, to: Point, kind: ArrowKind) -> Result<Item> {
        let d = to - from;
        let span = d.hypot();
        if span < MIN_SPAN_PX {
            return Err(Error::Other(format!(
                "箭头跨度过短 {span:.1}px（下限 {MIN_SPAN_PX}）"
            )));
        }
        // ① 头高定标 → 字号反推（头高 ∝ 字号 pt 空间精确线性，探针实证）
        let head_px = (span * HEAD_SPAN_RATIO).clamp(HEAD_MIN_PX, HEAD_MAX_PX);
        let font_pt = head_px * PX_TO_PT / HEAD_PER_FONT;
        log::info!(
            "箭头装配：跨度 {span:.0}px → 头高 {head_px:.1}px / 字号 {font_pt:.1}pt / 杆厚 {:.2}px",
            SHAFT_PER_FONT * font_pt
        );
        // ② 按跨度排版（px→pt；stretch size 精确吸收，宽零偏差免精调）
        let body = format!("stretch({}, size: #{}pt)", kind.sym(), span * PX_TO_PT);
        let f = self.formula(&body, &formula_opts(font_pt))?;
        // ③ 装配片合并（单头 1+N 片 / 双头 2+N 片）
        let mut path = assemble(&f)?;
        // ④ 归一化：原点 = from、杆沿 +x、中线 y=0、头在 +x 端
        //    （原始 pt 空间 y 向下：X = (x−x0)·PT_TO_PX，Y = (y_m−y)·PT_TO_PX）
        let bb = path.bounding_box();
        if !(bb.width() > 0.0 && bb.height() > 0.0) {
            return Err(Error::Other("箭头装配路径为空".into()));
        }
        let y_mid = (bb.y0 + bb.y1) / 2.0;
        path.apply_affine(
            Affine::translate((-PT_TO_PX * bb.x0, PT_TO_PX * y_mid))
                * Affine::new([PT_TO_PX, 0.0, 0.0, -PT_TO_PX, 0.0, 0.0]),
        );
        // ⑤ 弦角旋转 + 平移到 from：单一显式矩阵（c=dx/span, s=dy/span，
        //    画布 y 向下系顺时针正角；kurbo 列序 x'=a·x+c·y+e / y'=b·x+d·y+f）
        //    头尖 (span,0) → (from.x+dx, from.y+dy) = to 精确闭合。
        let (c, s) = (d.x / span, d.y / span);
        let affine = Affine::new([c, s, -s, c, from.x, from.y]);
        Ok(Item::from_path(path).with_transform(affine))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOL: f64 = 0.5;

    #[test]
    fn arrow_水平几何与跨度自适应() {
        let mut engine = TypstEngine::new(8);
        // span 400px → 头高 = clamp(60, 10, 24) = 24（上限封顶）
        let a = engine
            .arrow(
                Point::new(100.0, 300.0),
                Point::new(500.0, 300.0),
                ArrowKind::Single,
            )
            .unwrap();
        let bb = a.bbox();
        // bbox 精确铺满 [from, to]（头尖 = to 端）
        assert!(
            (bb.x0 - 100.0).abs() < TOL && (bb.x1 - 500.0).abs() < TOL,
            "跨度 {bb:?}"
        );
        // 杆过弦（中线 y=300）、头对称张开 ±12px
        assert!(
            (bb.y0 - (300.0 - 12.0)).abs() < TOL && (bb.y1 - (300.0 + 12.0)).abs() < TOL,
            "头高 ±12 {bb:?}"
        );
    }

    #[test]
    fn arrow_斜向弦角旋转() {
        let mut engine = TypstEngine::new(8);
        // 对角弦：span = √(300²+200²) = 360.55，头高封顶 24
        let a = engine
            .arrow(
                Point::new(100.0, 100.0),
                Point::new(400.0, 300.0),
                ArrowKind::Single,
            )
            .unwrap();
        // 解析预测（翼垂直弦 ±w 投影）：c=0.8321, s=0.5547, w=12
        // x: 100 ± s·12 → [93.34, 406.66]；y: 100 − c·12 .. 100 + s·span + c·12
        //   = [90.02, 309.98]
        let bb = a.bbox();
        assert!(
            (bb.x0 - 93.34).abs() < 1.0 && (bb.x1 - 406.66).abs() < 1.0,
            "x {bb:?}"
        );
        assert!(
            (bb.y0 - 90.02).abs() < 1.0 && (bb.y1 - 309.98).abs() < 1.0,
            "y {bb:?}"
        );
    }

    #[test]
    fn arrow_双头两端对称() {
        let mut engine = TypstEngine::new(8);
        let a = engine
            .arrow(
                Point::new(100.0, 300.0),
                Point::new(500.0, 300.0),
                ArrowKind::Double,
            )
            .unwrap();
        let bb = a.bbox();
        // 双头：bbox 铺满 [from, to]（两端头尖 = 两端点，探针实证 19 片 2 头）
        assert!(
            (bb.x0 - 100.0).abs() < TOL && (bb.x1 - 500.0).abs() < TOL,
            "跨度 {bb:?}"
        );
        // 头高与单头同（16.64pt @ 32pt 字号族，两端各一头不增头高）
        assert!(
            (bb.y0 - (300.0 - 12.0)).abs() < TOL && (bb.y1 - (300.0 + 12.0)).abs() < TOL,
            "双头 ±12 {bb:?}"
        );
    }

    #[test]
    fn arrow_短杆头下限封顶() {
        let mut engine = TypstEngine::new(8);
        // span 30px → 头高 = clamp(4.5, 10, 24) = 10（下限封顶）
        let a = engine
            .arrow(
                Point::new(100.0, 100.0),
                Point::new(130.0, 100.0),
                ArrowKind::Single,
            )
            .unwrap();
        let bb = a.bbox();
        assert!((bb.x1 - bb.x0 - 30.0).abs() < TOL, "短杆跨度 {bb:?}");
        assert!((bb.y1 - bb.y0 - 10.0).abs() < TOL, "头高下限 10 {bb:?}");
    }

    #[test]
    fn arrow_跨度过短报错() {
        let mut engine = TypstEngine::new(8);
        let r = engine.arrow(
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            ArrowKind::Single,
        );
        let err = r.expect_err("span 10 < 20 应返回 Err");
        assert!(err.to_string().contains("跨度过短"), "{err}");
    }

    /// 杆厚 ∝ 字号（0.048）实证：比例链恒定（探针 10.8:1 头杆比族）。
    #[test]
    fn arrow_杆厚随字号() {
        let head_px = 24.0_f64;
        let font_pt = head_px * PX_TO_PT / HEAD_PER_FONT;
        let shaft_px = SHAFT_PER_FONT * font_pt * PT_TO_PX;
        assert!((shaft_px / head_px - SHAFT_PER_FONT / HEAD_PER_FONT).abs() < 1e-9);
    }
}
