//! ffanim-typst：Typst 库嵌入（唯一接触面）。
//!
//! - `MathWorld`：进程内 World 实现（字体来自 typst-assets，主源为内存字符串）；
//! - [`TypstEngine`]：公式排版入口 + LRU 缓存（同公式二次排版 0 开销）；
//! - [`Formula`]：排版产物 → ffanim `Item` 适配层（字形/形状 → 贝塞尔路径）。
//!
//! 坐标约定：Formula 内部为 pt 单位、y 向下（与 Typst Frame 一致），
//! 缩放/居中由调用方通过 `Item::with_transform` 完成。
mod engine;
mod formula;
mod matching;

pub use engine::TypstEngine;
pub use formula::{Formula, FormulaOptions, FormulaPiece};
pub use matching::{MatchPlan, match_formulas};

#[cfg(test)]
mod tests {
    use super::*;
    use ffanim_core::Color;
    use ffanim_core::kurbo::Affine;
    use std::sync::Arc;

    fn opts() -> FormulaOptions {
        FormulaOptions {
            font_size_pt: 32.0,
            text_color: Color::new(240, 240, 240, 255),
        }
    }

    /// 排版一个基础公式，应有非空片段且尺寸为正。
    #[test]
    fn 公式排版非空() {
        let mut engine = TypstEngine::new(8);
        let f = engine.formula("x^2 + y^2 = z^2", &opts()).unwrap();
        assert!(!f.pieces.is_empty(), "应有字形片段");
        assert!(f.width_pt > 0.0 && f.height_pt > 0.0);
        // 每个片段路径非空
        for p in &f.pieces {
            assert!(!p.path.elements().is_empty(), "片段路径不应为空");
        }
    }

    /// fit_transform 应把缩放后的公式中心精确落在目标点。
    #[test]
    fn fit_transform缩放居中() {
        let mut engine = TypstEngine::new(8);
        let f = engine.formula("x^2 + y^2 = z^2", &opts()).unwrap();
        let (w, h) = (f.width_pt, f.height_pt);
        // 中心点 (w/2, h/2) 经变换后应落在 (480, 158)
        let t = f.fit_transform(480.0, 158.0, 1.5);
        let c = t * ffanim_core::kurbo::Point::new(w / 2.0, h / 2.0);
        assert!(
            (c.x - 480.0).abs() < 1e-6 && (c.y - 158.0).abs() < 1e-6,
            "中心: {c:?}"
        );
        // 左上角 (0,0) 应为 480 - 1.5w/2（缩放后居中，而非未缩放居中）
        let o = t * ffanim_core::kurbo::Point::new(0.0, 0.0);
        assert!(
            (o.x - (480.0 - 1.5 * w / 2.0)).abs() < 1e-6,
            "左上角 x: {}",
            o.x
        );
    }

    /// 缓存：同公式二次排版应返回同一 Arc（0 开销）。
    #[test]
    fn 二次排版零开销() {
        let mut engine = TypstEngine::new(8);
        let a = engine
            .formula("integral_0^oo e^(-x^2) dif x", &opts())
            .unwrap();
        let b = engine
            .formula("integral_0^oo e^(-x^2) dif x", &opts())
            .unwrap();
        assert!(Arc::ptr_eq(&a, &b), "缓存命中应返回同一 Arc");
    }

    /// 分数应产生描边或填充的分数线条形状片段（非字形）。
    #[test]
    fn 分数线形状() {
        let mut engine = TypstEngine::new(8);
        let f = engine.formula("a/b", &opts()).unwrap();
        assert!(
            f.pieces
                .iter()
                .any(|p| p.source_range.is_none() && p.text.is_none()),
            "分数线应为无源码区间/无文本的形状片段"
        );
    }

    /// 结构匹配：相同子式配对，不同部分进出。
    #[test]
    fn 公式结构匹配() {
        let mut engine = TypstEngine::new(8);
        let a = engine.formula("x^2 + y^2 = z^2", &opts()).unwrap();
        let b = engine.formula("x^2 - y^2 = 0", &opts()).unwrap();
        let plan = match_formulas(&a, &b);
        // x, 2, y, 2, = 至少 5 个字形配对
        assert!(plan.paired.len() >= 5, "配对数: {}", plan.paired.len());
        // 配对两边的片段文本一致
        for (i, j) in &plan.paired {
            assert_eq!(a.pieces[*i].text, b.pieces[*j].text);
        }
        // z^2 / + / 0 / - 未配对，进出各有所归
        assert_eq!(plan.leave_a.len() + plan.paired.len(), a.pieces.len());
        assert_eq!(plan.enter_b.len() + plan.paired.len(), b.pieces.len());
    }

    /// 非法公式应返回可读错误。
    #[test]
    fn 非法公式报错() {
        let mut engine = TypstEngine::new(8);
        // NaN 尺寸会产出空页面或错误，均不应 panic
        let bad = FormulaOptions {
            font_size_pt: f64::NAN,
            ..opts()
        };
        let _ = engine.formula("x", &bad);
    }

    /// 渲染冒烟：公式 → Items → CPU 后端 PNG（人工核验排版正确性）。
    #[test]
    fn 公式渲染出图() {
        use ffanim_core::DrawList;

        let mut engine = TypstEngine::new(8);
        let f = engine
            .formula("integral_0^oo e^(-x^2) dif x = sqrt(pi)/2", &opts())
            .unwrap();

        let (w, h) = (960u32, 540u32);
        let mut draw = DrawList::new(w, h, Color::new(14, 17, 22, 255));
        let scale = 2.0; // pt → px 放大便于目检
        let t = f.center_transform(f64::from(w) / 2.0, f64::from(h) / 2.0) * Affine::scale(scale);
        for item in f.to_items() {
            let transform = item.transform;
            draw.push(item.with_transform(t * transform));
        }

        use ffanim_render::RenderBackend;
        let mut cpu = ffanim_render::CpuBackend::new();
        let rgba = cpu.render(&draw);
        let out = std::path::Path::new("../../frames/formula_smoke.png");
        if let Some(dir) = out.parent() {
            std::fs::create_dir_all(dir).unwrap();
        }
        ffanim_render::save_png(out, w, h, &rgba).unwrap();
    }
}
