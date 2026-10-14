//! ffanim-typst：Typst 库嵌入（唯一接触面）。
//!
//! - `MathWorld`：进程内 World 实现（字体来自 typst-assets，主源为内存字符串）；
//! - [`TypstEngine`]：公式/富文本排版入口 + LRU 缓存（同源码二次排版 0 开销）；
//! - [`Formula`]：排版产物 → ffanim `Item` 适配层（字形/形状 → 贝塞尔路径）。
//!
//! 坐标约定：Formula 内部为 pt 单位、y 向下（与 Typst Frame 一致），
//! 缩放/居中由调用方通过 `Item::with_transform` 完成。
mod arrow;
mod assemble;
mod code;
mod decimal;
mod delimiter;
mod engine;
mod formula;
mod geom;
mod matching;
mod t2c;
mod text;

pub use arrow::ArrowKind;
pub use code::{CodeOptions, CodeTheme};
pub use decimal::DecimalNumberOptions;
pub use delimiter::Delimiter;
pub use engine::TypstEngine;
pub use formula::{Formula, FormulaOptions, FormulaPiece};
pub use matching::{MatchPlan, match_formulas};
pub use t2c::{build_t2c_markup, escape_markup};
pub use text::{TextOptions, find_cjk_font};

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

    // -----------------------------------------------------------------------
    // 富文本 Text（M8.2）
    // -----------------------------------------------------------------------

    fn text_opts(size: f64) -> TextOptions {
        TextOptions::new(size, Color::new(240, 240, 240, 255))
    }

    /// 文本缓存命中：同参数二次排版返回同一 Arc。
    #[test]
    fn text_缓存命中() {
        let mut engine = TypstEngine::new(8);
        let o = text_opts(24.0);
        let a = engine.text("ffanim", &o).unwrap();
        let b = engine.text("ffanim", &o).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "缓存命中应返回同一 Arc");
    }

    /// 同源码的 formula 与 text 缓存互不串扰（模式在缓存键中）。
    #[test]
    fn text与公式缓存不串() {
        let mut engine = TypstEngine::new(8);
        let o = text_opts(24.0);
        let f = engine.formula("x^2", &opts()).unwrap();
        let t = engine.text("x^2", &o).unwrap();
        assert!(!Arc::ptr_eq(&f, &t), "math/markup 模式不同，产物应不同");
    }

    /// 粗细混排 markup 编译成功且产物非空（`*…*` 走 LibertinusSerif-Bold）。
    #[test]
    fn text_粗体混排() {
        let mut engine = TypstEngine::new(8);
        let o = text_opts(24.0);
        let f = engine.text("普通 *bold* 普通", &o).unwrap();
        assert!(!f.pieces.is_empty());
        assert!(f.width_pt > 0.0 && f.height_pt > 0.0);
    }

    /// 行距选项生效：同文本行距大者整体更高（拉丁字体即可，不依赖系统 CJK）。
    #[test]
    fn text_行距影响高度() {
        let mut engine = TypstEngine::new(8);
        let loose = TextOptions {
            leading_em: 1.5,
            ..text_opts(20.0)
        };
        let tight = TextOptions {
            leading_em: 0.65,
            ..text_opts(20.0)
        };
        let a = engine.text("Hello\\\nWorld", &loose).unwrap();
        let b = engine.text("Hello\\\nWorld", &tight).unwrap();
        assert!(
            a.height_pt > b.height_pt,
            "{} vs {}",
            a.height_pt,
            b.height_pt
        );
    }

    /// 中英混排限宽换行：产物宽度不超过限宽 + margin（验收核心断言）。
    /// 需要系统中文字体，缺失时跳过。
    #[test]
    fn text_中英混排限宽不出框() {
        let Some((font, idx)) = find_cjk_font() else {
            eprintln!("跳过：未找到系统中文字体");
            return;
        };
        let mut engine = TypstEngine::new(8);
        engine.add_font_face(&font, idx).unwrap();
        let o = TextOptions {
            wrap_width_pt: Some(220.0),
            ..text_opts(18.0)
        };
        let f = engine
            .text(
                "数学动画引擎 ffanim 支持中英混排 mixed text，限宽后自动换行 \
                 line wrapping，绝不溢出画布。",
                &o,
            )
            .unwrap();
        // margin 左右各 2pt，页面宽 = 内容宽 + 4pt
        assert!(
            f.width_pt <= 224.1,
            "排版宽度 {} 超出限宽 220pt",
            f.width_pt
        );
        assert!(
            f.height_pt > 36.0,
            "多行文本高度应超过两行: {}",
            f.height_pt
        );
        assert!(!f.pieces.is_empty());
        // 片段含中文字形（text 字段有源文本）
        assert!(
            f.pieces.iter().any(|p| p
                .text
                .as_deref()
                .is_some_and(|t| t.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)))),
            "应有中文字形片段"
        );
    }

    // -----------------------------------------------------------------------
    // 代码块（P1-13）
    // -----------------------------------------------------------------------

    fn code_opts(size: f64) -> CodeOptions {
        CodeOptions::new(size)
    }

    const RUST_SNIPPET: &str = "// 注释 comment\nfn main() {\n    let x = 42;\n    let s = \"hello\";\n    println!(\"{}\", x);\n}";

    /// 语法高亮生效：关键字/字符串/数字/注释应呈多种颜色（深色主题）。
    #[test]
    fn code_语法高亮多色() {
        let mut engine = TypstEngine::new(8);
        let f = engine.code(RUST_SNIPPET, "rust", &code_opts(13.0)).unwrap();
        assert!(!f.pieces.is_empty());
        assert!(f.width_pt > 0.0 && f.height_pt > 0.0);
        let fills: std::collections::HashSet<String> = f
            .pieces
            .iter()
            .filter_map(|p| {
                p.fill
                    .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
            })
            .collect();
        assert!(
            fills.len() >= 3,
            "rust 片段应至少 3 种 token 色，实际 {fills:?}"
        );
        // 深色主题前景色应出现（普通标识符，piece fill 为小写 hex）
        assert!(
            fills.contains("#e6edf3"),
            "应含 ffanim-dark 前景色，实际 {fills:?}"
        );
    }

    /// 浅色主题：前景色为深色系（浅底语义）。
    #[test]
    fn code_浅色主题() {
        let mut engine = TypstEngine::new(8);
        let o = CodeOptions {
            theme: CodeTheme::Light,
            ..code_opts(13.0)
        };
        let f = engine.code("let x = 1;", "rust", &o).unwrap();
        let fills: Vec<String> = f
            .pieces
            .iter()
            .filter_map(|p| {
                p.fill
                    .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
            })
            .collect();
        assert!(
            fills.iter().any(|h| h == "#1f2328"),
            "应含 ffanim-light 前景色，实际 {fills:?}"
        );
    }

    /// 缓存：同 body/lang/theme 二次排版返回同一 Arc；换语言不串缓存。
    #[test]
    fn code_缓存与键隔离() {
        let mut engine = TypstEngine::new(8);
        let a = engine.code("let x = 1;", "rust", &code_opts(13.0)).unwrap();
        let b = engine.code("let x = 1;", "rust", &code_opts(13.0)).unwrap();
        assert!(Arc::ptr_eq(&a, &b), "缓存命中应返回同一 Arc");
        // 换主题 → 不同产物
        let c = engine
            .code(
                "let x = 1;",
                "rust",
                &CodeOptions {
                    theme: CodeTheme::Light,
                    ..code_opts(13.0)
                },
            )
            .unwrap();
        assert!(!Arc::ptr_eq(&a, &c), "主题不同缓存应隔离");
    }

    /// 正文含三反引号时围栏自动升级，排版不破块。
    #[test]
    fn code_围栏升级() {
        let mut engine = TypstEngine::new(8);
        let body = "a ```inline``` b\nfn x() {}";
        let f = engine.code(body, "text", &code_opts(12.0)).unwrap();
        assert!(!f.pieces.is_empty());
        // 行内容完整保留（verbatim）：含反引号子串的字形应存在
        let text: String = f.pieces.iter().filter_map(|p| p.text.as_deref()).collect();
        assert!(text.contains("```"), "反引号应原样保留: {text:?}");
    }

    /// 无语言标签 = 纯文本块，单一前景色。
    #[test]
    fn code_无语言纯文本() {
        let mut engine = TypstEngine::new(8);
        let f = engine.code("plain text 123", "", &code_opts(12.0)).unwrap();
        let fills: std::collections::HashSet<String> = f
            .pieces
            .iter()
            .filter_map(|p| {
                p.fill
                    .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
            })
            .collect();
        assert_eq!(fills.len(), 1, "纯文本应只有前景色: {fills:?}");
    }

    /// 渲染冒烟：代码块 → CPU 后端 PNG（人工核验高亮观感）。
    #[test]
    fn code_渲染出图() {
        use ffanim_core::DrawList;

        let mut engine = TypstEngine::new(8);
        let f = engine.code(RUST_SNIPPET, "rust", &code_opts(13.0)).unwrap();

        let (w, h) = (960u32, 540u32);
        let mut draw = DrawList::new(w, h, Color::new(14, 17, 22, 255));
        let t = f.center_transform(f64::from(w) / 2.0, f64::from(h) / 2.0) * Affine::scale(3.0);
        for item in f.to_items() {
            let transform = item.transform;
            draw.push(item.with_transform(t * transform));
        }

        use ffanim_render::RenderBackend;
        let mut cpu = ffanim_render::CpuBackend::new();
        let rgba = cpu.render(&draw);
        let out = std::path::Path::new("../../frames/code_smoke.png");
        ffanim_render::save_png(out, w, h, &rgba).unwrap();
    }
}
