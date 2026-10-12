//! 结构匹配：`MatchPlan` 与 LCS 配对算法（TransformMatchingTex 基座）。

use crate::formula::Formula;

/// 两公式的片段匹配结果，供场景层调度 Transform / FadeOut / FadeIn。
#[derive(Debug, Clone)]
pub struct MatchPlan {
    /// 配对片段：(a 下标, b 下标) —— 将做 Transform 形变。
    pub paired: Vec<(usize, usize)>,
    /// a 中未配对（将 FadeOut）。
    pub leave_a: Vec<usize>,
    /// b 中未配对（将 FadeIn）。
    pub enter_b: Vec<usize>,
}

/// 按片段源文本的最长公共子序列（LCS）配对两个公式的片段。
///
/// 与 manim `TransformMatchingTex` 的精神一致：相同子式（如 `x^2`、`=`）
/// 跨公式保留并形变，其余淡出/淡入。形状片段（分数线等，无文本）恒不配对。
pub fn match_formulas(a: &Formula, b: &Formula) -> MatchPlan {
    let ta: Vec<Option<&str>> = a.pieces.iter().map(|p| p.text.as_deref()).collect();
    let tb: Vec<Option<&str>> = b.pieces.iter().map(|p| p.text.as_deref()).collect();
    let (n, m) = (ta.len(), tb.len());

    // dp[i][j] = ta[i..] 与 tb[j..] 的 LCS 长度
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if ta[i].is_some() && ta[i] == tb[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    // 回溯取配对
    let mut paired = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if ta[i].is_some() && ta[i] == tb[j] {
            paired.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }

    let used_a: std::collections::HashSet<usize> = paired.iter().map(|x| x.0).collect();
    let used_b: std::collections::HashSet<usize> = paired.iter().map(|x| x.1).collect();
    MatchPlan {
        leave_a: (0..n).filter(|k| !used_a.contains(k)).collect(),
        enter_b: (0..m).filter(|k| !used_b.contains(k)).collect(),
        paired,
    }
}
