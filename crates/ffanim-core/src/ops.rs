//! 路径布尔运算：[`BoolOp`] + [`Item::boolean`]（manim `Union/Intersection/Exclusion` 对标）。
//!
//! 数据流三步：BezPath 平坦化为轮廓集 → i_overlay 浮点布尔 → 折线重建为 BezPath。
//! 结果是普通 Item，create/transform 等动画零改动直接适用。

use crate::{Error, Item, Result, Shape};
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use kurbo::{BezPath, PathEl, Point};

/// 布尔运算种类（subject = self，clip = other）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    /// 并集 A ∪ B。
    Union,
    /// 交集 A ∩ B。
    Intersection,
    /// 差集 A − B。
    Difference,
    /// 异或（属于其一但不同时属于两者）。
    Xor,
}

impl BoolOp {
    fn to_rule(self) -> OverlayRule {
        match self {
            Self::Union => OverlayRule::Union,
            Self::Intersection => OverlayRule::Intersect,
            Self::Difference => OverlayRule::Difference,
            Self::Xor => OverlayRule::Xor,
        }
    }
}

/// 每段贝塞尔的平坦化采样数：像素域下弦差 ~0.02px，布尔结果视觉无损。
const FLATTEN_SEGMENTS: usize = 24;

impl Item {
    /// 路径布尔：self op other → 新 Item（折线路径，样式继承 self）。
    ///
    /// 仅支持 Path 形状（Group/Image 返回错误）；不相交时返回空路径。
    /// 布尔在平坦化多边形上进行——贝塞尔特征点（如圆弧极值）不在输出锚点中保留。
    pub fn boolean(&self, op: BoolOp, other: &Item) -> Result<Self> {
        let (Shape::Path(a), Shape::Path(b)) = (&self.shape, &other.shape) else {
            return Err(Error::Other("布尔运算仅支持 Path 形状".into()));
        };
        let subj = flatten_path(a);
        let clip = flatten_path(b);
        let shapes = subj.overlay(&clip, op.to_rule(), FillRule::NonZero);
        Ok(Item {
            id: 0,
            shape: Shape::Path(rebuild_path(&shapes)),
            paint: self.paint.clone(),
            transform: self.transform,
            clip: None,
        })
    }
}

/// BezPath → 闭合轮廓集（每条子路径一个 contour）。
///
/// cubic/quad 按固定段数等参采样；少于 3 点的退化轮廓丢弃；
/// ClosePath 与「末点=首点」两种闭合写法均归一为闭合 contour。
fn flatten_path(path: &BezPath) -> Vec<Vec<[f64; 2]>> {
    let mut contours: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut cur: Vec<[f64; 2]> = Vec::new();
    let mut start = Point::ZERO;
    let mut last = Point::ZERO;
    for el in path.elements() {
        match el {
            PathEl::MoveTo(p) => {
                if cur.len() >= 3 {
                    contours.push(std::mem::take(&mut cur));
                } else {
                    cur.clear();
                }
                cur.push([p.x, p.y]);
                start = *p;
                last = *p;
            }
            PathEl::LineTo(p) => {
                // ClosePath 后接 LineTo 的罕见序列：隐式以 start 起新轮廓
                if cur.is_empty() {
                    cur.push([start.x, start.y]);
                }
                cur.push([p.x, p.y]);
                last = *p;
            }
            PathEl::QuadTo(c, p) => {
                for i in 1..=FLATTEN_SEGMENTS {
                    let t = i as f64 / FLATTEN_SEGMENTS as f64;
                    let u = 1.0 - t;
                    let x = u * u * last.x + 2.0 * u * t * c.x + t * t * p.x;
                    let y = u * u * last.y + 2.0 * u * t * c.y + t * t * p.y;
                    cur.push([x, y]);
                }
                last = *p;
            }
            PathEl::CurveTo(c1, c2, p) => {
                for i in 1..=FLATTEN_SEGMENTS {
                    let t = i as f64 / FLATTEN_SEGMENTS as f64;
                    let u = 1.0 - t;
                    let x = u * u * u * last.x
                        + 3.0 * u * u * t * c1.x
                        + 3.0 * u * t * t * c2.x
                        + t * t * t * p.x;
                    let y = u * u * u * last.y
                        + 3.0 * u * u * t * c1.y
                        + 3.0 * u * t * t * c2.y
                        + t * t * t * p.y;
                    cur.push([x, y]);
                }
                last = *p;
            }
            PathEl::ClosePath => {
                if cur.len() >= 3 {
                    contours.push(std::mem::take(&mut cur));
                } else {
                    cur.clear();
                }
                last = start;
            }
        }
    }
    if cur.len() >= 3 {
        contours.push(cur);
    }
    contours
}

/// i_overlay Shapes → BezPath：每个 contour 一条闭合折线子路径。
///
/// 外圈与洞方向相反，NonZero 填充下洞自然挖空（vello 默认 NonZero）。
fn rebuild_path(shapes: &[Vec<Vec<[f64; 2]>>]) -> BezPath {
    let mut path = BezPath::new();
    for shape in shapes {
        for contour in shape {
            let Some(&first) = contour.first() else {
                continue;
            };
            path.move_to(Point::new(first[0], first[1]));
            for p in &contour[1..] {
                path.line_to(Point::new(p[0], p[1]));
            }
            path.close_path();
        }
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Shape;
    use kurbo::Shape as _;
    use kurbo::{Circle, Rect};

    /// 构造矩形 Item（像素域）。
    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Item {
        Item::rect(Rect::new(x0, y0, x1, y1)).with_fill(crate::Color::new(255, 0, 0, 255))
    }

    /// 折线面积（shoelace，经 kurbo Shape::area）。
    fn area(item: &Item) -> f64 {
        let Shape::Path(p) = &item.shape else {
            panic!("应为路径");
        };
        p.area()
    }

    #[test]
    fn 四则面积正确() {
        // (0,0)-(2,2) 与 (1,1)-(3,3)：交=1，并=7，差=3，异或=6
        let (a, b) = (rect(0.0, 0.0, 2.0, 2.0), rect(1.0, 1.0, 3.0, 3.0));
        let cases = [
            (BoolOp::Intersection, 1.0),
            (BoolOp::Union, 7.0),
            (BoolOp::Difference, 3.0),
            (BoolOp::Xor, 6.0),
        ];
        for (op, expected) in cases {
            let got = area(&a.boolean(op, &b).unwrap());
            assert!(
                (got - expected).abs() < 1e-3,
                "{op:?} 面积 {got} != {expected}"
            );
        }
    }

    #[test]
    fn 差集产生洞结构() {
        // (0,0)-(4,4) 挖掉 (1,1)-(3,3)：面积 12，两个 contour（外圈+洞）
        let (a, b) = (rect(0.0, 0.0, 4.0, 4.0), rect(1.0, 1.0, 3.0, 3.0));
        let r = a.boolean(BoolOp::Difference, &b).unwrap();
        assert!((area(&r) - 12.0).abs() < 1e-3);
        let Shape::Path(p) = &r.shape else {
            panic!("应为路径");
        };
        let subpaths = p
            .elements()
            .iter()
            .filter(|el| matches!(el, PathEl::MoveTo(_)))
            .count();
        assert_eq!(subpaths, 2, "应有外圈与洞两条子路径");
    }

    #[test]
    fn 贝塞尔圆参与布尔面积近似正确() {
        // 圆 r=40 ∩ 右半平面矩形：半圆 π·40²/2 ≈ 2513（平坦化误差 <1%）
        let circle = Item::circle(Circle::new((50.0, 50.0), 40.0));
        let half = rect(50.0, 0.0, 200.0, 100.0);
        let got = area(&circle.boolean(BoolOp::Intersection, &half).unwrap());
        let expected = std::f64::consts::PI * 1600.0 / 2.0;
        assert!(
            (got - expected).abs() / expected < 0.01,
            "半圆面积 {got} != {expected}"
        );
    }

    #[test]
    fn 不相交交集为空路径() {
        let (a, b) = (rect(0.0, 0.0, 1.0, 1.0), rect(10.0, 10.0, 11.0, 11.0));
        let r = a.boolean(BoolOp::Intersection, &b).unwrap();
        assert_eq!(area(&r), 0.0);
    }

    #[test]
    fn group输入返回错误() {
        let g = Item::group(vec![rect(0.0, 0.0, 1.0, 1.0)]);
        assert!(rect(0.0, 0.0, 2.0, 2.0).boolean(BoolOp::Union, &g).is_err());
        assert!(g.boolean(BoolOp::Union, &rect(0.0, 0.0, 2.0, 2.0)).is_err());
    }

    #[test]
    fn 样式继承自self() {
        let a = rect(0.0, 0.0, 2.0, 2.0);
        let b = rect(1.0, 1.0, 3.0, 3.0);
        let r = a.boolean(BoolOp::Union, &b).unwrap();
        assert_eq!(r.paint, a.paint);
    }
}
