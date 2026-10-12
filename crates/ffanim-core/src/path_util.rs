//! 路径工具：部分路径提取（Create/Write 动画的基础）。
//!
//! 按贝塞尔段数比例分配进度（与 manim 的点数比例策略等价量级），
//! 多子路径（如文本的多个字形）统一计段、顺序绘制。

use kurbo::{Affine, BezPath, PathEl, Point};

/// 提取路径的前 `alpha` 比例（∈\[0,1\]，超界钳制）。
///
/// - `alpha = 1.0` 返回完整路径；
/// - 最后一个未完成的段通过 de Casteljau 拆分取前半段；
/// - 跨子路径顺序推进（Create 一段文本会逐字形画出）。
pub fn partial_path(path: &BezPath, alpha: f64) -> BezPath {
    let alpha = alpha.clamp(0.0, 1.0);
    if alpha >= 1.0 {
        return path.clone();
    }
    if alpha <= 0.0 {
        return BezPath::new();
    }

    // 按 MoveTo 切分子路径
    let mut subpaths: Vec<Vec<PathEl>> = vec![Vec::new()];
    for el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) && !subpaths.last().unwrap().is_empty() {
            subpaths.push(Vec::new());
        }
        subpaths.last_mut().unwrap().push(*el);
    }
    subpaths.retain(|sp| !sp.is_empty());

    // 统计总段数（LineTo/QuadTo/CurveTo/ClosePath 各记 1 段）
    let segments_of = |sp: &[PathEl]| -> usize {
        sp.iter()
            .filter(|el| {
                matches!(
                    el,
                    PathEl::LineTo(_)
                        | PathEl::QuadTo(_, _)
                        | PathEl::CurveTo(_, _, _)
                        | PathEl::ClosePath
                )
            })
            .count()
    };
    let total: usize = subpaths.iter().map(|sp| segments_of(sp)).sum();
    let mut budget = alpha * total as f64;

    let mut out = BezPath::new();
    for sp in &subpaths {
        if budget <= 0.0 {
            break;
        }
        let mut started = false;
        let mut prev = Point::ZERO;
        for el in sp {
            let seg_len = match el {
                PathEl::LineTo(_)
                | PathEl::QuadTo(_, _)
                | PathEl::CurveTo(_, _, _)
                | PathEl::ClosePath => 1.0,
                PathEl::MoveTo(_) => 0.0,
            };
            if seg_len == 0.0 {
                let PathEl::MoveTo(p) = el else {
                    unreachable!()
                };
                out.move_to(*p);
                started = true;
                prev = *p;
                continue;
            }
            if !started {
                // 子路径以非 MoveTo 开头（理论不发生），防御性处理
                out.move_to(prev);
                started = true;
            }
            if budget >= seg_len {
                push_full(&mut out, *el);
                budget -= seg_len;
            } else {
                // 部分段：按本地进度拆分
                let local = budget / seg_len;
                push_partial(&mut out, prev, *el, local);
                budget = 0.0;
                break;
            }
            if let Some(end) = el_end(el) {
                prev = end;
            }
        }
    }
    out
}

fn push_full(out: &mut BezPath, el: PathEl) {
    match el {
        PathEl::ClosePath => out.close_path(),
        other => out.push(other),
    }
}

/// 段的终点（ClosePath 无显式终点，由调用方的 prev 维护语义，返回 None 跳过）。
fn el_end(el: &PathEl) -> Option<Point> {
    match el {
        PathEl::LineTo(p) | PathEl::QuadTo(_, p) | PathEl::CurveTo(_, _, p) | PathEl::MoveTo(p) => {
            Some(*p)
        }
        PathEl::ClosePath => None,
    }
}

/// 取段的前 `t` 部分（de Casteljau 拆分）。
fn push_partial(out: &mut BezPath, prev: Point, el: PathEl, t: f64) {
    match el {
        PathEl::LineTo(p) => out.line_to(lerp_pt(prev, p, t)),
        PathEl::QuadTo(c, p) => {
            // 二次贝塞尔拆分
            let q0 = lerp_pt(prev, c, t);
            let q1 = lerp_pt(q0, lerp_pt(c, p, t), t);
            out.quad_to(q0, q1);
        }
        PathEl::CurveTo(c1, c2, p) => {
            // 三次贝塞尔 de Casteljau 前半段
            let a = lerp_pt(prev, c1, t);
            let b = lerp_pt(c1, c2, t);
            let c = lerp_pt(c2, p, t);
            let d = lerp_pt(a, b, t);
            let e = lerp_pt(b, c, t);
            let f = lerp_pt(d, e, t);
            out.curve_to(a, d, f);
        }
        PathEl::ClosePath => {
            // Close 视作回 MoveTo 起点的直线段；起点未知时按原样闭合
            out.line_to(lerp_pt(prev, prev, t));
            out.close_path();
        }
        PathEl::MoveTo(_) => {}
    }
}

fn lerp_pt(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// 路径逐点插值（元素序列完全一致时可用，用于 Transform 形变）。
pub fn lerp_path(a: &BezPath, b: &BezPath, t: f64) -> Option<BezPath> {
    let (ea, eb) = (a.elements(), b.elements());
    if ea.len() != eb.len() {
        return None;
    }
    let mut out = BezPath::new();
    for (x, y) in ea.iter().zip(eb.iter()) {
        match (x, y) {
            (PathEl::MoveTo(p1), PathEl::MoveTo(p2)) => out.move_to(lerp_pt(*p1, *p2, t)),
            (PathEl::LineTo(p1), PathEl::LineTo(p2)) => out.line_to(lerp_pt(*p1, *p2, t)),
            (PathEl::QuadTo(c1, p1), PathEl::QuadTo(c2, p2)) => {
                out.quad_to(lerp_pt(*c1, *c2, t), lerp_pt(*p1, *p2, t))
            }
            (PathEl::CurveTo(a1, b1, p1), PathEl::CurveTo(a2, b2, p2)) => out.curve_to(
                lerp_pt(*a1, *a2, t),
                lerp_pt(*b1, *b2, t),
                lerp_pt(*p1, *p2, t),
            ),
            (PathEl::ClosePath, PathEl::ClosePath) => out.close_path(),
            _ => return None, // 元素种类不一致，不可形变
        }
    }
    Some(out)
}

/// 仿射矩阵逐系数插值（平移/缩放/旋转小角度场景视觉上足够）。
pub fn lerp_affine(a: &Affine, b: &Affine, t: f64) -> Affine {
    let (x, y) = (a.as_coeffs(), b.as_coeffs());
    Affine::new([
        x[0] + (y[0] - x[0]) * t,
        x[1] + (y[1] - x[1]) * t,
        x[2] + (y[2] - x[2]) * t,
        x[3] + (y[3] - x[3]) * t,
        x[4] + (y[4] - x[4]) * t,
        x[5] + (y[5] - x[5]) * t,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> BezPath {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((100.0, 0.0));
        p.line_to((50.0, -80.0));
        p.close_path();
        p
    }

    #[test]
    fn 完整与空提取() {
        let t = triangle();
        assert_eq!(partial_path(&t, 1.0).elements().len(), t.elements().len());
        assert_eq!(partial_path(&t, 0.0).elements().len(), 0);
    }

    #[test]
    fn 部分提取的段数递增() {
        let t = triangle();
        // 3 段（2 line + close）
        let a_quarter = partial_path(&t, 0.2).elements().len();
        let a_half = partial_path(&t, 0.5).elements().len();
        let a_full3 = partial_path(&t, 0.9).elements().len();
        assert_eq!(a_quarter, 2); // move + 半条线
        assert_eq!(a_half, 3); // move + 1.5 条线
        assert_eq!(a_full3, 5); // move + 3 段 + 部分 close 段
    }

    #[test]
    fn 逐点形变需要相同元素序列() {
        let a = triangle();
        // 构造平移后的同构路径（kurbo 0.13 的 BezPath 无 transform 方法）
        let mut b = BezPath::new();
        for el in a.elements() {
            match *el {
                PathEl::MoveTo(p) => b.move_to((p.x + 50.0, p.y + 50.0)),
                PathEl::LineTo(p) => b.line_to((p.x + 50.0, p.y + 50.0)),
                other => b.push(other),
            }
        }
        assert!(lerp_path(&a, &b, 0.5).is_some());

        let c = triangle();
        let mut d = triangle();
        d.move_to((10.0, 10.0)); // 元素数不同
        assert!(lerp_path(&c, &d, 0.5).is_none());
    }

    #[test]
    fn 文本式多子路径顺序推进() {
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((10.0, 0.0));
        p.move_to((20.0, 0.0));
        p.line_to((30.0, 0.0));
        // 50%：恰好画完第一个子路径（move+line），第二个子路径尚未开始
        let out = partial_path(&p, 0.5);
        assert_eq!(out.elements().len(), 2);
        // 75%：第二个子路径画一半（move + 完整 line + move + 部分 line = 4）
        let out = partial_path(&p, 0.75);
        assert_eq!(out.elements().len(), 4);
    }
}
