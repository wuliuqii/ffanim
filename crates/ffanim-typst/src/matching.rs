//! 结构匹配：`MatchPlan` 与序列 diff 配对算法（TransformMatchingTex 基座）。
//!
//! 算法 = Myers diff（similar）+ move 配对：
//! 1. Equal 段逐位配对（不变子式）；
//! 2. Delete/Insert（含 Replace 段拆解）候选之间按同文本 FIFO 二部配对——
//!    同文本的删除/插入对识别为**移动**（index 允许乱序），这是对纯 LCS 的
//!    关键升级：`a+b=c → b+a=c` 中被交换的子式做 Transform 而非消失重现；
//! 3. 仍未配对的进 leave_a/enter_b。
//!
//! 形状片段（无源文本，如分数线）用侧别唯一占位符防跨配对，恒不配对。

use crate::formula::Formula;
use similar::{DiffOp, TextDiff};
use std::collections::{HashMap, VecDeque};

// MatchPlan 上移统一至 ffanim-core::match_shape（match_shapes 几何配对共用同型），
// 此处 re-export 保持 `ffanim_typst::MatchPlan` 公开路径不变。
pub use ffanim_core::match_shape::MatchPlan;

/// 按片段源文本 diff 配对两个公式（Myers + move 配对）。
///
/// 与 manim `TransformMatchingTex` 的精神一致：相同子式（如 `x^2`、`=`）
/// 跨公式保留并形变（移动的子式配对为 Transform），其余淡出/淡入。
/// 形状片段（分数线等，无文本）恒不配对。
pub fn match_formulas(a: &Formula, b: &Formula) -> MatchPlan {
    // 无文本的形状片段用侧别唯一占位符：跨序列永不相等 → 恒进 leave/enter
    let token = |p: &crate::formula::FormulaPiece, i: usize, side: char| match &p.text {
        Some(t) => t.clone(),
        None => format!("\u{0}{side}{i}"),
    };
    let ta: Vec<String> = a
        .pieces
        .iter()
        .enumerate()
        .map(|(i, p)| token(p, i, 'a'))
        .collect();
    let tb: Vec<String> = b
        .pieces
        .iter()
        .enumerate()
        .map(|(i, p)| token(p, i, 'b'))
        .collect();
    // similar::TextDiff::diff_slices 接收 &[&str]（T=str unsized），转引用切片
    let refs_a: Vec<&str> = ta.iter().map(String::as_str).collect();
    let refs_b: Vec<&str> = tb.iter().map(String::as_str).collect();
    let diff = TextDiff::configure().diff_slices(&refs_a, &refs_b);

    // Equal 段逐位配对；Delete/Insert/Replace 拆成候选，交给 move 二部配对
    let mut paired: Vec<(usize, usize)> = Vec::new();
    let mut dels: Vec<usize> = Vec::new();
    let mut ins: Vec<usize> = Vec::new();
    for op in diff.ops() {
        match *op {
            DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                paired.extend((0..len).map(|k| (old_index + k, new_index + k)));
            }
            DiffOp::Delete {
                old_index, old_len, ..
            } => {
                dels.extend(old_index..old_index + old_len);
            }
            DiffOp::Insert {
                new_index, new_len, ..
            } => {
                ins.extend(new_index..new_index + new_len);
            }
            DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                dels.extend(old_index..old_index + old_len);
                ins.extend(new_index..new_index + new_len);
            }
        }
    }

    // move 二部配对：同文本的删除/插入候选按出现序 FIFO 配对
    let mut by_text: HashMap<&str, VecDeque<usize>> = HashMap::new();
    for &j in &ins {
        by_text.entry(tb[j].as_str()).or_default().push_back(j);
    }
    let mut leave_a = Vec::new();
    let mut matched_b: Vec<usize> = Vec::new();
    for &i in &dels {
        let hit = by_text.get_mut(ta[i].as_str()).and_then(|q| q.pop_front());
        match hit {
            Some(j) => {
                paired.push((i, j));
                matched_b.push(j);
            }
            None => leave_a.push(i),
        }
    }
    let matched: std::collections::HashSet<usize> = matched_b.into_iter().collect();
    let enter_b: Vec<usize> = ins
        .iter()
        .copied()
        .filter(|j| !matched.contains(j))
        .collect();

    // 输出按 (a 下标, b 下标) 排序保证确定性（move 配对天然乱序）
    paired.sort_unstable();
    MatchPlan {
        paired,
        leave_a,
        enter_b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formula::{Formula, FormulaPiece};

    /// 按源文本序列构造测试公式（width/height 置 0，配对算法只看 text）。
    fn formula_of(texts: &[Option<&str>]) -> Formula {
        Formula {
            width_pt: 0.0,
            height_pt: 0.0,
            pieces: texts
                .iter()
                .map(|t| FormulaPiece {
                    path: ffanim_core::kurbo::BezPath::new(),
                    fill: None,
                    stroke: None,
                    source_range: Some((0, 0)),
                    text: t.map(String::from),
                })
                .collect(),
        }
    }

    #[test]
    fn 交换子式识别为移动() {
        // a+b=c → b+a=c：纯 LCS 会把 a、b 判成"删除+新增"，
        // move 配对应识别 a、b 交换为 Transform
        let a = formula_of(&[Some("a"), Some("+"), Some("b"), Some("="), Some("c")]);
        let b = formula_of(&[Some("b"), Some("+"), Some("a"), Some("="), Some("c")]);
        let plan = match_formulas(&a, &b);
        assert!(
            plan.paired.contains(&(0, 2)) && plan.paired.contains(&(2, 0)),
            "交换的 a/b 应配对为移动: {plan:?}"
        );
        assert!(
            plan.paired.contains(&(1, 1))
                && plan.paired.contains(&(3, 3))
                && plan.paired.contains(&(4, 4)),
            "+ = c 应原位配对: {plan:?}"
        );
        assert!(
            plan.leave_a.is_empty() && plan.enter_b.is_empty(),
            "不应有残留: {plan:?}"
        );
    }

    #[test]
    fn 替换子式原位配对() {
        // x^2 + 1 → x^2 - 1：TransformMatchingTex 经典用例
        let a = formula_of(&[Some("x^2"), Some("+"), Some("1")]);
        let b = formula_of(&[Some("x^2"), Some("-"), Some("1")]);
        let plan = match_formulas(&a, &b);
        assert_eq!(plan.paired, vec![(0, 0), (2, 2)]);
        assert_eq!(plan.leave_a, vec![1]);
        assert_eq!(plan.enter_b, vec![1]);
    }

    #[test]
    fn 形状片段恒不配对() {
        // 分数线（无文本）两侧各一：侧别占位符互异，不得互相配对
        let a = formula_of(&[Some("1"), None, Some("2")]);
        let b = formula_of(&[Some("3"), None, Some("2")]);
        let plan = match_formulas(&a, &b);
        assert!(plan.paired.contains(&(2, 2)), "相同文本 2 应配对: {plan:?}");
        assert!(
            !plan.paired.iter().any(|&(x, y)| x == 1 || y == 1),
            "形状片段（占位符）不应配对: {plan:?}"
        );
        assert_eq!(plan.leave_a, vec![0, 1]);
        assert_eq!(plan.enter_b, vec![0, 1]);
    }
}
