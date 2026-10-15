//! 平面直角坐标系：数据坐标 ↔ 画布像素的双向映射 + 轴/网格/函数图像生成。
//!
//! 设计要点：
//! - `Axes` 是纯几何辅助结构（Copy，不进渲染列表），轴/网格/图像由它生成
//!   为普通 [`Item`] 后走常规动画词汇（Create/FadeIn/Transform...）；
//! - 生成的路径直接烘入像素坐标（c2p 一次性变换），与场景其余对象同构；
//! - 数据 y 轴向上、画布 y 轴向下，翻转在 c2p 内完成。

use crate::{Color, Item, Style};
use kurbo::{BezPath, Point};

/// 平面直角坐标系（数据坐标范围 + 画布占位）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axes {
    /// x 数据范围 (min, max)。
    pub x_range: (f64, f64),
    /// y 数据范围 (min, max)。
    pub y_range: (f64, f64),
    /// 数据区域在画布上占据的尺寸（px）。
    pub size: (f64, f64),
    /// 数据区域中心在画布上的位置（px）。
    pub center: (f64, f64),
}

impl Axes {
    /// 以默认占位（400x300，中心 (200, 150)）创建，通常随后链式修正。
    pub fn new(x_range: (f64, f64), y_range: (f64, f64)) -> Self {
        Self {
            x_range,
            y_range,
            size: (400.0, 300.0),
            center: (200.0, 150.0),
        }
    }

    /// 数据区域占位尺寸（px）。
    pub fn with_size(mut self, w: f64, h: f64) -> Self {
        self.size = (w, h);
        self
    }

    /// 数据区域中心位置（px）。
    pub fn with_center(mut self, cx: f64, cy: f64) -> Self {
        self.center = (cx, cy);
        self
    }

    /// 数据坐标 → 画布像素（y 翻转）。
    pub fn c2p(&self, x: f64, y: f64) -> Point {
        let (x0, x1) = self.x_range;
        let (y0, y1) = self.y_range;
        let (w, h) = self.size;
        let (cx, cy) = self.center;
        Point::new(
            cx - w / 2.0 + (x - x0) / (x1 - x0) * w,
            cy - h / 2.0 + (y1 - y) / (y1 - y0) * h,
        )
    }

    /// 画布像素 → 数据坐标（c2p 的逆映射）。
    pub fn p2c(&self, p: Point) -> (f64, f64) {
        let (x0, x1) = self.x_range;
        let (y0, y1) = self.y_range;
        let (w, h) = self.size;
        let (cx, cy) = self.center;
        (
            x0 + (p.x - cx + w / 2.0) / w * (x1 - x0),
            y0 + (cy + h / 2.0 - p.y) / h * (y1 - y0),
        )
    }

    /// 坐标轴（x 轴 + y 轴 + 单位刻度），样式取自主题 Style。
    ///
    /// 轴线画在数据原点处；原点不在范围内时退到区域边缘（manim 同款行为）。
    pub fn to_item(&self, style: &Style) -> Item {
        let axis_color = style
            .stroke
            .as_ref()
            .and_then(|s| s.parse().ok())
            .unwrap_or(Color::new(200, 200, 200, 255));
        let (w, h) = self.size;
        let (x0, x1) = self.x_range;
        let (y0, y1) = self.y_range;
        // 轴线所在数据坐标：0 在范围内用 0，否则贴边
        let ax_y = 0.0_f64.clamp(y0.min(y1), y0.max(y1));
        let ay_x = 0.0_f64.clamp(x0.min(x1), x0.max(x1));

        let mut path = BezPath::new();
        // x 轴与 y 轴
        let xa = self.c2p(x0.min(x1), ax_y);
        let xb = self.c2p(x0.max(x1), ax_y);
        let ya = self.c2p(ay_x, y0.min(y1));
        let yb = self.c2p(ay_x, y0.max(y1));
        path.move_to(xa);
        path.line_to(xb);
        path.move_to(ya);
        path.line_to(yb);

        // 单位刻度：x 轴竖直短线、y 轴水平短线（各 8px）
        let tick = 4.0;
        let mut t = x0.ceil();
        while t <= x1 {
            if t.abs() > 1e-9 {
                let p = self.c2p(t, ax_y);
                path.move_to((p.x, p.y - tick));
                path.line_to((p.x, p.y + tick));
            }
            t += 1.0;
        }
        let mut t = y0.ceil();
        while t <= y1 {
            if t.abs() > 1e-9 {
                let p = self.c2p(ay_x, t);
                path.move_to((p.x - tick, p.y));
                path.line_to((p.x + tick, p.y));
            }
            t += 1.0;
        }
        let _ = (w, h); // 尺寸经 c2p 间接生效
        Item::from_path(path)
            .with_stroke(axis_color)
            .with_stroke_width(f64::from(style.stroke_width))
    }

    /// NumberPlane 风格网格（每 step 一个数据单位一条线），弱化色由调用方给定。
    pub fn grid_item(&self, color: Color, step: f64) -> Item {
        let (x0, x1) = self.x_range;
        let (y0, y1) = self.y_range;
        let mut path = BezPath::new();
        let mut t = (x0 / step).ceil() * step;
        while t <= x1 {
            let a = self.c2p(t, y0.min(y1));
            let b = self.c2p(t, y0.max(y1));
            path.move_to(a);
            path.line_to(b);
            t += step;
        }
        let mut t = (y0 / step).ceil() * step;
        while t <= y1 {
            let a = self.c2p(x0.min(x1), t);
            let b = self.c2p(x0.max(x1), t);
            path.move_to(a);
            path.line_to(b);
            t += step;
        }
        Item::from_path(path)
            .with_stroke(color)
            .with_stroke_width(1.0)
    }

    /// 函数图像：在 x_range 上等距采样，Catmull-Rom 平滑为三次贝塞尔
    /// （控制点取相邻点 1/6 切线）。
    pub fn plot(&self, f: impl Fn(f64) -> f64, x_range: (f64, f64)) -> Item {
        const SAMPLES: usize = 200;
        let (xa, xb) = x_range;
        let pts: Vec<Point> = (0..=SAMPLES)
            .map(|i| {
                let x = xa + (xb - xa) * i as f64 / SAMPLES as f64;
                self.c2p(x, f(x))
            })
            .collect();
        let mut path = BezPath::new();
        path.move_to(pts[0]);
        for i in 0..pts.len() - 1 {
            let p0 = pts[i.saturating_sub(1)];
            let p1 = pts[i];
            let p2 = pts[i + 1];
            let p3 = pts[(i + 2).min(pts.len() - 1)];
            let c1 = p1 + (p2 - p0) / 6.0;
            let c2 = p2 - (p3 - p1) / 6.0;
            path.curve_to(c1, c2, p2);
        }
        Item::from_path(path)
    }
}

impl Color {
    /// 返回指定 alpha 的新颜色（网格等弱化色用）。
    pub fn with_alpha(self, a: u8) -> Self {
        Self { a, ..self }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Shape;

    fn test_axes() -> Axes {
        Axes::new((-2.0, 2.0), (-1.0, 1.0))
            .with_size(400.0, 200.0)
            .with_center(300.0, 200.0)
    }

    #[test]
    fn c2p_p2c往返一致() {
        let axes = test_axes();
        for (x, y) in [(-1.5, 0.5), (0.0, 0.0), (2.0, -1.0), (1.0, 1.0)] {
            let p = axes.c2p(x, y);
            let (cx, cy) = axes.p2c(p);
            assert!((cx - x).abs() < 1e-9, "x 往返不一致: {x} → {cx}");
            assert!((cy - y).abs() < 1e-9, "y 往返不一致: {y} → {cy}");
        }
    }

    #[test]
    fn c2p端点与y翻转() {
        let axes = test_axes();
        // 数据原点 (−2, 1)（y 最大）→ 像素左上角
        let tl = axes.c2p(-2.0, 1.0);
        assert_eq!(tl, Point::new(100.0, 100.0));
        // 数据 (2, −1) → 像素右下角
        let br = axes.c2p(2.0, -1.0);
        assert_eq!(br, Point::new(500.0, 300.0));
        // y 轴向上：数据 y 越大像素 y 越小
        assert!(axes.c2p(0.0, 1.0).y < axes.c2p(0.0, -1.0).y);
    }

    #[test]
    fn plot经过采样点且平滑() {
        let axes = test_axes();
        let item = axes.plot(|x| x * x, (-2.0, 2.0));
        let Shape::Path(path) = &item.shape else {
            panic!("应为路径");
        };
        // 200 段曲线：move_to + 200 curve_to
        let els = path.elements();
        assert_eq!(els.len(), 201);
        // 起点 = f(−2) = 4 的像素位置
        let expected = axes.c2p(-2.0, 4.0);
        assert!(matches!(els[0], kurbo::PathEl::MoveTo(p) if (p.x - expected.x).abs() < 1e-9));
    }

    #[test]
    fn 轴线包含原点与边缘退化() {
        let mut style = crate::Style {
            fill: None,
            stroke: None,
            stroke_width: 2.0,
        };
        style.stroke = None;
        // 无 stroke 时走兜底色，仍应生成
        let axes = test_axes();
        let Shape::Path(_) = axes.to_item(&style).shape else {
            panic!("应为路径");
        };

        // 原点在范围内：x 轴画在 y=0
        let axes_in = Axes::new((-2.0, 2.0), (-1.0, 1.0));
        let item = axes_in.to_item(&style);
        let Shape::Path(p) = &item.shape else {
            panic!()
        };
        // 第一条线是 x 轴，两端 y 像素相同且对应数据 y=0
        let els = p.elements();
        let Some(kurbo::PathEl::MoveTo(a)) = els.first() else {
            panic!()
        };
        let (cx, cy) = axes_in.p2c(*a);
        assert!(cy.abs() < 1e-9, "x 轴应在数据 y=0 处");

        // 原点不在 y 范围内：x 轴退化到下边缘
        let axes_out = Axes::new((-2.0, 2.0), (1.0, 2.0));
        let Shape::Path(p) = &axes_out.to_item(&style).shape else {
            panic!()
        };
        let Some(kurbo::PathEl::MoveTo(a)) = p.elements().first() else {
            panic!()
        };
        let (_, cy) = axes_out.p2c(*a);
        assert!((cy - 1.0).abs() < 1e-9, "x 轴应退到 y 下边缘");
        let _ = cx;
    }

    #[test]
    fn 网格线数正确() {
        let axes = Axes::new((-2.0, 2.0), (-1.0, 1.0));
        let item = axes.grid_item(Color::new(0, 0, 0, 60), 1.0);
        let Shape::Path(p) = &item.shape else {
            panic!()
        };
        // x 方向：−2,−1,0,1,2 → 5 条；y 方向：−1,0,1 → 3 条；每条 move+line
        let moves = p
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 8);
    }
}
