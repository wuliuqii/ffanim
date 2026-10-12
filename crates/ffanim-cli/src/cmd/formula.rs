//! 公式结构匹配诊断：`ffanim diff`（TransformMatchingTex 调度计划预检）。

use clap::Parser;
use ffanim::Color;
use ffanim::typst::{FormulaOptions, TypstEngine, match_formulas};

const FG: Color = Color::new(230, 233, 239, 255);

/// 对比两个公式的片段匹配计划：哪些子式形变（Transform）、哪些淡出/淡入。
///
/// 供场景作者在写 TransformMatchingTex 场景前预检匹配质量
/// （move 配对会以乱序下标出现，表示"识别为移动"）。
#[derive(Parser)]
pub struct DiffArgs {
    /// 公式 A（Typst 数学语法，如 "x^2 + 1"）
    pub a: String,

    /// 公式 B（Typst 数学语法，如 "x^2 - 1"）
    pub b: String,
}

/// `ffanim diff` 入口：排版两公式 → 打印 MatchPlan。
pub fn diff(args: DiffArgs) -> ffanim::Result<()> {
    let mut eng = TypstEngine::new(16);
    let opts = FormulaOptions {
        font_size_pt: 32.0,
        text_color: FG,
    };
    let f1 = eng.formula(&args.a, &opts)?;
    let f2 = eng.formula(&args.b, &opts)?;
    let plan = match_formulas(&f1, &f2);

    println!("配对（Transform）: {}", plan.paired.len());
    for &(i, j) in &plan.paired {
        println!(
            "  [{i}] {}  ->  [{j}] {}",
            piece_text(&f1, i),
            piece_text(&f2, j)
        );
    }
    println!("淡出（leave_a）: {}", plan.leave_a.len());
    for &i in &plan.leave_a {
        println!("  [{i}] {}", piece_text(&f1, i));
    }
    println!("淡入（enter_b）: {}", plan.enter_b.len());
    for &j in &plan.enter_b {
        println!("  [{j}] {}", piece_text(&f2, j));
    }
    Ok(())
}

/// 取公式的第 i 个片段源文本（形状片段返回占位）。
fn piece_text(f: &ffanim::typst::Formula, i: usize) -> String {
    f.pieces[i].text.clone().unwrap_or_else(|| "<形状>".into())
}
