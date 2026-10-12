//! 对象模型：一切可视对象的统一表示。
//!
//! 设计要点（见 docs/02-架构设计.md §3.3）：
//! - 单一 `Item` 表示，无 Mobject/VMobject 分裂，`Group` 即 Item；
//! - 变换挂在 Item 上，不烘焙进点阵，插值 = 属性插值 + 可选路径 morph；
//! - `DrawList` 是渲染层唯一输入，GPU/CPU 后端消费同一表示。

use crate::Color;
use kurbo::{Affine, BezPath, Circle, Rect};
// kurbo::Shape 与本模块的 Shape 枚举重名，匿名导入
use kurbo::Shape as _;

/// 对象唯一标识（时间轴引用与缓存键的基础）。
pub type ItemId = u64;

/// 填充/描边样式（已解析为具体颜色，主题解析发生在构建期）。
#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: f64,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: 4.0,
        }
    }
}

impl Paint {
    /// 整体透明度缩放（alpha 分量相乘）。
    pub fn with_opacity(&self, opacity: f64) -> Self {
        let scale = |c: Option<Color>| {
            c.map(|mut c| {
                c.a = ((c.a as f64 * opacity).round() as i64).clamp(0, 255) as u8;
                c
            })
        };
        Self {
            fill: scale(self.fill),
            stroke: scale(self.stroke),
            stroke_width: self.stroke_width,
        }
    }
}

/// 形状来源：路径或组合。
#[derive(Debug, Clone)]
pub enum Shape {
    /// 三次贝塞尔路径（扁平点阵的 kurbo 表示，锚点+双控制柄）。
    Path(BezPath),
    /// 组合：子 Item 随父 Item 一起变换。
    Group(Vec<Item>),
}

/// 可视对象。
#[derive(Debug, Clone)]
pub struct Item {
    pub id: ItemId,
    pub shape: Shape,
    pub paint: Paint,
    pub transform: Affine,
}

impl Item {
    /// 从路径构造（无样式，需链式设置）。
    pub fn from_path(path: BezPath) -> Self {
        Self {
            id: 0,
            shape: Shape::Path(path),
            paint: Paint::default(),
            transform: Affine::IDENTITY,
        }
    }

    /// 矩形。
    pub fn rect(rect: Rect) -> Self {
        let mut path = BezPath::new();
        path.push(kurbo::PathEl::MoveTo(rect.origin()));
        path.push(kurbo::PathEl::LineTo((rect.x1, rect.y0).into()));
        path.push(kurbo::PathEl::LineTo((rect.x1, rect.y1).into()));
        path.push(kurbo::PathEl::LineTo((rect.x0, rect.y1).into()));
        path.close_path();
        Self::from_path(path)
    }

    /// 圆（按容差近似为四段三次贝塞尔）。
    pub fn circle(circle: Circle) -> Self {
        Self::from_path(circle.to_path(0.1))
    }

    /// 正多边形。
    pub fn regular_polygon(center: kurbo::Point, radius: f64, sides: usize) -> Self {
        assert!(sides >= 3, "多边形至少 3 条边");
        let mut path = BezPath::new();
        for i in 0..sides {
            let angle =
                (i as f64) * std::f64::consts::TAU / sides as f64 - std::f64::consts::FRAC_PI_2;
            let p = kurbo::Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            );
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        // 显式闭合：保证描边在起点处有完整拐角连接
        path.close_path();
        Self::from_path(path)
    }

    pub fn with_id(mut self, id: ItemId) -> Self {
        self.id = id;
        self
    }

    pub fn with_fill(mut self, color: Color) -> Self {
        self.paint.fill = Some(color);
        self
    }

    pub fn with_stroke(mut self, color: Color) -> Self {
        self.paint.stroke = Some(color);
        self
    }

    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.paint.stroke_width = width;
        self
    }

    pub fn with_transform(mut self, transform: Affine) -> Self {
        self.transform = transform;
        self
    }

    /// 平移快捷方式。
    pub fn with_translate(self, dx: f64, dy: f64) -> Self {
        self.with_transform(Affine::translate((dx, dy)))
    }
}

/// 一帧的绘制指令（渲染层唯一输入）。
#[derive(Debug, Clone)]
pub struct DrawList {
    pub width: u32,
    pub height: u32,
    pub background: Color,
    pub items: Vec<Item>,
}

impl DrawList {
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            width,
            height,
            background,
            items: Vec::new(),
        }
    }

    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 正多边形闭合路径() {
        let item = Item::regular_polygon((100.0, 100.0).into(), 50.0, 3);
        let Shape::Path(path) = &item.shape else {
            panic!("应为路径");
        };
        let elements: Vec<_> = path.elements().to_vec();
        // 3 顶点：move_to + 2 line_to + close = 4
        assert_eq!(elements.len(), 4);
        assert!(matches!(elements[0], kurbo::PathEl::MoveTo(_)));
    }

    #[test]
    fn 组合即条目() {
        let group = Item {
            id: 1,
            shape: Shape::Group(vec![Item::circle(Circle::new((0.0, 0.0), 10.0))]),
            paint: Paint::default(),
            transform: Affine::IDENTITY,
        };
        let Shape::Group(children) = group.shape else {
            panic!("应为组合");
        };
        assert_eq!(children.len(), 1);
    }
}
