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

/// 提取路径的段级子路径：保留第 `⌈t0×总段数⌉..⌈t1×总段数⌉` 段（t ∈ \[0,1\] 比例）。
///
/// 段粒度裁剪（整段进出，不做段内拆分），用于 ShowPassingFlash 类"沿路径局部
/// 高亮"。首个保留段若处于子路径中间，补 MoveTo 保证路径合法（描边从该段起点画起）。
///
/// # Panics
/// `t0 >= t1` 或越界 \[0,1\] 时 panic。
pub fn span_path(path: &BezPath, t0: f64, t1: f64) -> BezPath {
    assert!(
        (0.0..=1.0).contains(&t0) && (0.0..=1.0).contains(&t1) && t0 < t1,
        "span_path 需 0 ≤ t0 < t1 ≤ 1，得到 [{t0}, {t1}]"
    );
    // 子路径切分与段计数（与 partial_path 同规则）
    let mut subpaths: Vec<Vec<PathEl>> = vec![Vec::new()];
    for el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) && !subpaths.last().unwrap().is_empty() {
            subpaths.push(Vec::new());
        }
        subpaths.last_mut().unwrap().push(*el);
    }
    subpaths.retain(|sp| !sp.is_empty());
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
    let lo = (t0 * total as f64).ceil() as usize;
    let hi = (t1 * total as f64).ceil() as usize;

    let mut out = BezPath::new();
    let mut seg = 0usize;
    let mut prev = Point::ZERO;
    let mut collecting = false;
    for sp in &subpaths {
        let mut first_in_sub = true;
        let sub_start = sp.iter().find_map(|el| match el {
            PathEl::MoveTo(p) => Some(*p),
            _ => None,
        });
        for el in sp {
            if let PathEl::MoveTo(p) = el {
                prev = *p;
                first_in_sub = true;
                continue;
            }
            if seg >= lo && seg < hi {
                if first_in_sub || !collecting {
                    out.move_to(prev);
                }
                if let (PathEl::ClosePath, Some(s)) = (el, sub_start) {
                    // 闭合段保留：显式画回子路径起点（新路径起点已变，
                    // close_path 会错闭合到段起点而非子路径起点）
                    out.line_to(s);
                } else {
                    push_full(&mut out, *el);
                }
                collecting = true;
            } else {
                collecting = false;
            }
            if let Some(end) = el_end(el) {
                prev = end;
            }
            seg += 1;
            first_in_sub = false;
        }
    }
    out
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

/// 各段与其累计弧长（段序即路径顺序；MoveTo 不产生段）。
fn cumulative_arclens(path: &BezPath) -> (Vec<(kurbo::PathSeg, f64)>, f64) {
    use kurbo::ParamCurveArclen;
    const ACCURACY: f64 = 1e-6;
    let mut segs = Vec::new();
    let mut total = 0.0;
    for seg in path.segments() {
        total += seg.arclen(ACCURACY);
        segs.push((seg, total));
    }
    (segs, total)
}

/// 路径总弧长（pt；空路径 / 纯 MoveTo 返回 0）。
pub fn path_length(path: &BezPath) -> f64 {
    cumulative_arclens(path).1
}

/// 路径按弧长比例采样点（`alpha` ∈ \[0,1\]，**匀速参数化**，manim
/// `point_at_length` 对应；MoveAlongPath 逐帧位移的基础）。
///
/// 空路径 / 零长路径返回 (0,0)；`alpha` 超界钳制。
pub fn sample_path(path: &BezPath, alpha: f64) -> Point {
    use kurbo::{ParamCurve, ParamCurveArclen};
    const ACCURACY: f64 = 1e-6;
    let target_len = alpha.clamp(0.0, 1.0);
    let (segs, _) = cumulative_arclens(path);
    let Some(&(_, grand)) = segs.last() else {
        return Point::ZERO;
    };
    if grand <= ACCURACY {
        return segs[0].0.eval(0.0);
    }
    // 目标弧长所在段（首个累计弧长 ≥ 目标；alpha=1 命中末段）
    let idx = segs
        .partition_point(|&(_, cum)| cum < target_len * grand)
        .min(segs.len() - 1);
    let (seg, cum) = segs[idx];
    let prev = if idx == 0 { 0.0 } else { segs[idx - 1].1 };
    let local = if cum - prev <= ACCURACY {
        0.0
    } else {
        seg.inv_arclen(target_len * grand - prev, ACCURACY)
    };
    seg.eval(local.clamp(0.0, 1.0))
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

    #[test]
    fn 段级子路径提取() {
        let t = triangle(); // move + 2 line + close = 3 段
        // [0, 1/3)：第 1 段（move + line）
        assert_eq!(span_path(&t, 0.0, 1.0 / 3.0).elements().len(), 2);
        // [1/3, 2/3)：仅第 2 段（补 MoveTo + line = 2 元素）
        let mid = span_path(&t, 1.0 / 3.0, 2.0 / 3.0);
        assert_eq!(mid.elements().len(), 2, "{:?}", mid.elements());
        let Some(PathEl::MoveTo(p)) = mid.elements().first() else {
            panic!("首元素应为 MoveTo");
        };
        // 描边语义：MoveTo 补到段起点（第 2 段 = (100,0)→(50,-80)），
        // 补到子路径起点会画出错误线段
        assert_eq!(*p, Point::new(100.0, 0.0), "中段应从段起点补 MoveTo");
        // [0, 1)：全部 4 元素
        assert_eq!(span_path(&t, 0.0, 1.0).elements().len(), 4);
        // 断开的区间：[2/3, 1) 保留第 3 段（close）
        let tail = span_path(&t, 2.0 / 3.0, 1.0);
        assert_eq!(tail.elements().len(), 2, "{:?}", tail.elements());
    }

    #[test]
    #[should_panic(expected = "span_path 需")]
    fn 段级子路径非法区间报错() {
        let t = triangle();
        let _ = span_path(&t, 0.5, 0.5);
    }

    // ---- M9.10：弧长参数化采样（MoveAlongPath 基础）----

    #[test]
    fn 弧长采样直线匀速() {
        let mut p = BezPath::new();
        p.move_to((10.0, 20.0));
        p.line_to((110.0, 20.0));
        assert!((path_length(&p) - 100.0).abs() < 1e-6);
        // 起点 / 终点 / 中点（弧长 = 线性参数化）
        assert_eq!(sample_path(&p, 0.0), Point::new(10.0, 20.0));
        let mid = sample_path(&p, 0.5);
        assert!((mid.x - 60.0).abs() < 1e-6 && (mid.y - 20.0).abs() < 1e-6);
        let end = sample_path(&p, 1.0);
        assert!((end.x - 110.0).abs() < 1e-6);
        // 超界钳制
        assert_eq!(sample_path(&p, -1.0), Point::new(10.0, 20.0));
        assert_eq!(sample_path(&p, 2.0), Point::new(110.0, 20.0));
    }

    #[test]
    fn 弧长采样折线按长度占比() {
        // 两段折线：第一段长 60，第二段长 80，总长 140
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.line_to((60.0, 0.0));
        p.line_to((60.0, 80.0));
        assert!((path_length(&p) - 140.0).abs() < 1e-3);
        // alpha = 30/140 ≈ 0.2143：恰在第一段中点（30,0）——匀速而非按段数
        let q = sample_path(&p, 30.0 / 140.0);
        assert!((q.x - 30.0).abs() < 1e-3 && q.y.abs() < 1e-3, "{q:?}");
        // alpha = 100/140：落在第二段已走 40 处 → (60, 40)
        let q = sample_path(&p, 100.0 / 140.0);
        assert!(
            (q.x - 60.0).abs() < 1e-3 && (q.y - 40.0).abs() < 1e-3,
            "{q:?}"
        );
    }

    #[test]
    fn 弧长采样空路径与零长() {
        assert_eq!(sample_path(&BezPath::new(), 0.5), Point::ZERO);
        let mut dot = BezPath::new();
        dot.move_to((5.0, 5.0)); // 纯 MoveTo 无段
        assert!((path_length(&dot)).abs() < 1e-9);
        assert_eq!(sample_path(&dot, 0.5), Point::ZERO);
    }

    #[test]
    fn 弧长采样曲线段() {
        // 三次贝塞尔 (0,0)→(100,0) 控制点拉高：中点弧长参数化不等于参数中点，
        // 但应落在曲线上（y > 0）且端点精确
        let mut p = BezPath::new();
        p.move_to((0.0, 0.0));
        p.curve_to((30.0, -90.0), (70.0, -90.0), (100.0, 0.0));
        assert_eq!(sample_path(&p, 0.0), Point::new(0.0, 0.0));
        let end = sample_path(&p, 1.0);
        assert!((end.x - 100.0).abs() < 1e-6 && end.y.abs() < 1e-6);
        let mid = sample_path(&p, 0.5);
        assert!(mid.y < -10.0, "弧长中点应在鼓出段上: {mid:?}");
    }
}
