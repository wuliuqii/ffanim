//! 任意片段的结构匹配形变（manim `TransformMatchingShapes` 对应）。
//!
//! 与 `match_formulas`（ffanim-typst，源文本 diff 配对）互补：文本缺失的路径片段
//! （SVG 字形、几何图形）按**几何签名**配对——签名 = 平移归零 + 0.01pt 精度
//! 序列化，同形异位必配对（位置烘进坐标或挂 transform 均可），异形不配对。
//!
//! 算法 = 按签名 FIFO 二部配对：b 侧按签名建队列，a 侧逐个取队首——
//! 重复件按出现序对应，输出确定性排序（与文本无关，天然允许乱序移动）。

use crate::item::{Item, Shape};
use kurbo::{BezPath, PathEl, Shape as _};
use std::collections::{HashMap, VecDeque};

/// 两序列片段的匹配结果，供场景层调度 Transform / FadeOut / FadeIn。
///
/// （原定义于 ffanim-typst，随 match_shapes 上移统一：文本配对与几何配对共用。）
#[derive(Debug, Clone)]
pub struct MatchPlan {
    /// 配对片段：(a 下标, b 下标) —— 将做 Transform 形变（含移动配对，index 允许乱序）。
    pub paired: Vec<(usize, usize)>,
    /// a 中未配对（将 FadeOut）。
    pub leave_a: Vec<usize>,
    /// b 中未配对（将 FadeIn）。
    pub enter_b: Vec<usize>,
}

/// 路径几何签名：平移归零 + 0.01pt 精度序列化（同形异位 → 同签名）。
///
/// 空路径返回 None（不参与配对）。
pub fn shape_signature(path: &BezPath) -> Option<String> {
    if path.elements().is_empty() {
        return None;
    }
    let bb = path.bounding_box();
    if !bb.x0.is_finite() {
        return None;
    }
    let px = |p: kurbo::Point| format!("{:.2},{:.2}", p.x - bb.x0, p.y - bb.y0);
    let mut sig = String::new();
    for el in path.elements() {
        match *el {
            PathEl::MoveTo(p) => {
                sig.push('M');
                sig.push_str(&px(p));
            }
            PathEl::LineTo(p) => {
                sig.push('L');
                sig.push_str(&px(p));
            }
            PathEl::QuadTo(c, p) => {
                sig.push('Q');
                sig.push_str(&px(c));
                sig.push(' ');
                sig.push_str(&px(p));
            }
            PathEl::CurveTo(c1, c2, p) => {
                sig.push('C');
                sig.push_str(&px(c1));
                sig.push(' ');
                sig.push_str(&px(c2));
                sig.push(' ');
                sig.push_str(&px(p));
            }
            PathEl::ClosePath => sig.push('Z'),
        }
    }
    Some(sig)
}

/// 按几何签名配对两组散件（粒度由调用方决定：逐字形 / 逐图形均可）。
///
/// 只看 base path 几何且**平移无关**（outline_text/SVG 的烘焙位移、arrange 的
/// transform 挂位均适用）；样式（颜色/描边）不参与签名，Group/Image 不参与配对。
pub fn match_shapes(a: &[Item], b: &[Item]) -> MatchPlan {
    let sig_of = |it: &Item| match &it.shape {
        Shape::Path(p) => shape_signature(p),
        _ => None,
    };

    // b 侧按签名建 FIFO 队列（重复件按出现序对应）
    let mut queues: HashMap<String, VecDeque<usize>> = HashMap::new();
    for (j, it) in b.iter().enumerate() {
        if let Some(s) = sig_of(it) {
            queues.entry(s).or_default().push_back(j);
        }
    }

    let mut paired: Vec<(usize, usize)> = Vec::new();
    let mut leave_a: Vec<usize> = Vec::new();
    let mut used = vec![false; b.len()];
    for (i, it) in a.iter().enumerate() {
        let hit = sig_of(it)
            .as_deref()
            .and_then(|s| queues.get_mut(s))
            .and_then(VecDeque::pop_front);
        match hit {
            Some(j) => {
                used[j] = true;
                paired.push((i, j));
            }
            None => leave_a.push(i),
        }
    }
    let enter_b: Vec<usize> = (0..b.len()).filter(|&j| !used[j]).collect();
    paired.sort_unstable();

    let (na, nb) = (a.len(), b.len());
    log::info!(
        "match_shapes: a={na} 片段 / b={nb} 片段 → 配对 {}（形变含异位移动）、淡出 {}、淡入 {}",
        paired.len(),
        leave_a.len(),
        enter_b.len()
    );
    MatchPlan {
        paired,
        leave_a,
        enter_b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::{Affine, Circle, Rect};

    /// 圆形散件（base path 在原点，位置挂 transform）。
    fn circle_at(x: f64) -> Item {
        Item::circle(Circle::new((0.0, 0.0), 30.0)).with_transform(Affine::translate((x, 0.0)))
    }

    /// 方形散件（base path 随位置烘焙，用于对照"烘焙位移不配对"）。
    fn rect_baked(x: f64) -> Item {
        Item::rect(Rect::new(x, 0.0, x + 60.0, 60.0))
    }

    #[test]
    fn 同形异位配对_位置挂transform() {
        let a = vec![circle_at(100.0), circle_at(300.0)];
        let b = vec![circle_at(500.0), circle_at(700.0)];
        let plan = match_shapes(&a, &b);
        assert_eq!(plan.paired, vec![(0, 0), (1, 1)], "{plan:?}");
        assert!(plan.leave_a.is_empty() && plan.enter_b.is_empty());
    }

    #[test]
    fn 同形即配对_位置烘焙或transform均可() {
        // 签名平移归零：位置烘进 path 坐标（outline_text/SVG 产物即如此）与挂
        // transform 两种写法等价，同形异位一律配对
        let a = vec![rect_baked(0.0)];
        let b = vec![rect_baked(100.0)];
        let plan = match_shapes(&a, &b);
        assert_eq!(plan.paired, vec![(0, 0)], "{plan:?}");
    }

    #[test]
    fn 异形不配对() {
        let a = vec![circle_at(0.0)];
        let b = vec![rect_baked(0.0)];
        let plan = match_shapes(&a, &b);
        assert!(plan.paired.is_empty());
        assert_eq!(plan.leave_a, vec![0]);
        assert_eq!(plan.enter_b, vec![0]);
    }

    #[test]
    fn 乱序交换配对为移动() {
        // a=[方,圆] → b=[圆,方]：两对均为异位移动（与文本 diff 不同，天然乱序）
        let sq = |x: f64| rect_baked(x);
        let a = vec![sq(0.0), circle_at(100.0)];
        let b = vec![circle_at(200.0), sq(300.0)];
        let plan = match_shapes(&a, &b);
        assert_eq!(plan.paired, vec![(0, 1), (1, 0)], "{plan:?}");
    }

    #[test]
    fn 重复件按出现序顺序配对() {
        let a = vec![circle_at(0.0), circle_at(100.0)];
        let b = vec![circle_at(200.0), circle_at(300.0)];
        let plan = match_shapes(&a, &b);
        assert_eq!(plan.paired, vec![(0, 0), (1, 1)]);
    }

    #[test]
    fn 组合与图片不参与配对() {
        let grp = Item::group(vec![circle_at(0.0)]);
        let a = vec![grp.clone()];
        let b = vec![grp.clone()];
        let plan = match_shapes(&a, &b);
        assert!(plan.paired.is_empty());
        assert_eq!(plan.leave_a, vec![0]);
        assert_eq!(plan.enter_b, vec![0]);
    }

    #[test]
    fn 空路径无签名() {
        assert_eq!(shape_signature(&BezPath::new()), None);
    }
}
