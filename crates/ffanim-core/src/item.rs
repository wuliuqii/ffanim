//! 对象模型：一切可视对象的统一表示。
//!
//! 设计要点（见 docs/02-架构设计.md §3.3）：
//! - 单一 `Item` 表示，无 Mobject/VMobject 分裂，`Group` 即 Item；
//! - 变换挂在 Item 上，不烘焙进点阵，插值 = 属性插值 + 可选路径 morph；
//! - `DrawList` 是渲染层唯一输入，GPU/CPU 后端消费同一表示。

use crate::{Color, Error, Result};
use kurbo::{Affine, BezPath, Circle, Rect, Vec2};
// kurbo::Shape 与本模块的 Shape 枚举重名，匿名导入
use kurbo::Shape as _;

/// 对象唯一标识（时间轴引用与缓存键的基础）。
pub type ItemId = u64;

/// 填充/描边样式（已解析为具体颜色，主题解析发生在构建期）。
#[derive(Debug, Clone, PartialEq)]
pub struct Paint {
    /// 填充色（None = 不填充）。
    pub fill: Option<Color>,
    /// 描边色（None = 不描边）。
    pub stroke: Option<Color>,
    /// 描边宽度（像素）。
    pub stroke_width: f64,
    /// 整体透明度系数（图片对象专用；路径对象透明度承载在颜色 alpha）。
    pub opacity: f64,
}

impl Default for Paint {
    fn default() -> Self {
        Self {
            fill: None,
            stroke: None,
            stroke_width: 4.0,
            opacity: 1.0,
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
            opacity: self.opacity * opacity,
        }
    }
}

/// 位图资源（预乘 RGBA8，`Arc` 共享避免逐帧拷贝）。
#[derive(Clone)]
pub struct ImageAsset {
    /// 预乘 alpha 的 RGBA8 像素（行优先）。
    pub rgba: std::sync::Arc<[u8]>,
    /// 宽（像素）。
    pub width: u32,
    /// 高（像素）。
    pub height: u32,
}

impl std::fmt::Debug for ImageAsset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 像素数据可能巨大，只打印尺寸
        f.debug_struct("ImageAsset")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

impl ImageAsset {
    /// 从非预乘 RGBA8 像素构造（构造时一次性转预乘，长度不符返回错误）。
    pub fn from_rgba8(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self> {
        let expected = (width.saturating_mul(height) as usize) * 4;
        if rgba.len() != expected {
            return Err(Error::Other(format!(
                "图片数据长度不符: 期望 {expected} 字节（{width}x{height}x4），实际 {}",
                rgba.len()
            )));
        }
        let mut data = rgba;
        for px in data.chunks_exact_mut(4) {
            let a = u16::from(px[3]);
            px[0] = (u16::from(px[0]) * a / 255) as u8;
            px[1] = (u16::from(px[1]) * a / 255) as u8;
            px[2] = (u16::from(px[2]) * a / 255) as u8;
        }
        Ok(Self {
            rgba: data.into(),
            width,
            height,
        })
    }
}

/// 形状来源：路径或组合。
#[derive(Debug, Clone)]
pub enum Shape {
    /// 三次贝塞尔路径（扁平点阵的 kurbo 表示，锚点+双控制柄）。
    Path(BezPath),
    /// 组合：子 Item 随父 Item 一起变换。
    Group(Vec<Item>),
    /// 位图图片（预乘 RGBA8，尺寸见 [`ImageAsset`]）。
    Image(ImageAsset),
}

/// 可视对象。
#[derive(Debug, Clone)]
pub struct Item {
    /// 对象唯一标识（0 = 未注册，注册后由 Timeline 分配）。
    pub id: ItemId,
    /// 形状（路径或组合）。
    pub shape: Shape,
    /// 填充/描边样式。
    pub paint: Paint,
    /// 仿射变换（挂在 Item 上，不烘焙进点阵）。
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

    /// 从非预乘 RGBA8 像素构造图片对象（构造时一次性转预乘）。
    pub fn from_image(rgba: Vec<u8>, width: u32, height: u32) -> Result<Self> {
        let asset = ImageAsset::from_rgba8(rgba, width, height)?;
        Ok(Self {
            id: 0,
            shape: Shape::Image(asset),
            paint: Paint::default(),
            transform: Affine::IDENTITY,
        })
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

    /// 椭圆。
    pub fn ellipse(center: impl Into<kurbo::Point>, rx: f64, ry: f64) -> Self {
        Self::from_path(kurbo::Ellipse::new(center, Vec2::new(rx, ry), 0.0).to_path(0.1))
    }

    /// 圆弧：从 start_deg 到 end_deg（度），**视觉上逆时针**扫过（manim 语义；
    /// 屏幕 y 向下，实现层面 kurbo 数学角取负）。
    pub fn arc(center: impl Into<kurbo::Point>, radius: f64, start_deg: f64, end_deg: f64) -> Self {
        assert!(end_deg != start_deg, "圆弧起止角相同");
        let start = start_deg.to_radians();
        let sweep = (end_deg - start_deg).to_radians();
        // y 向下：视觉逆时针 = 数学顺时针 = 扫角取负
        let a = kurbo::Arc::new(center, Vec2::new(radius, radius), -start, -sweep, 0.0);
        Self::from_path(a.to_path(0.1))
    }

    /// 线段。
    pub fn line(a: impl Into<kurbo::Point>, b: impl Into<kurbo::Point>) -> Self {
        let mut path = BezPath::new();
        path.move_to(a.into());
        path.line_to(b.into());
        Self::from_path(path)
    }

    /// 虚线线段：dash/gap 交替沿 a→b 铺满。
    pub fn dashed_line(
        a: impl Into<kurbo::Point>,
        b: impl Into<kurbo::Point>,
        dash: f64,
        gap: f64,
    ) -> Self {
        let (a, b) = (a.into(), b.into());
        assert!(dash > 0.0 && gap > 0.0, "dash/gap 必须为正");
        let d = b - a;
        let len = d.hypot();
        assert!(len > 1e-6, "虚线起止点重合");
        let dir = d / len;
        let mut path = BezPath::new();
        let mut s = 0.0;
        while s < len {
            let e = (s + dash).min(len);
            path.move_to(a + dir * s);
            path.line_to(a + dir * e);
            s = e + gap;
        }
        Self::from_path(path)
    }

    /// 箭头：线段 + 闭合三角头（头长自适应 = min(线长/4, 32px)，半角 25°）。
    /// 三角头需填充：建议 `with_stroke(c).with_fill(c)` 同色。
    pub fn arrow(a: impl Into<kurbo::Point>, b: impl Into<kurbo::Point>) -> Self {
        let (a, b) = (a.into(), b.into());
        let d = b - a;
        let len = d.hypot();
        assert!(len > 1e-6, "箭头起止点重合");
        let dir = d / len;
        let head_len = (len * 0.25).min(32.0);
        let head_w = head_len * 0.466; // 半角 25°（tan 25° ≈ 0.466）
        let base = b - dir * head_len;
        let n = Vec2::new(-dir.y, dir.x);
        let mut path = BezPath::new();
        path.move_to(a);
        path.line_to(base); // 线段止于头部底边，避免穿出
        path.move_to(b);
        path.line_to(base + n * head_w);
        path.line_to(base - n * head_w);
        path.close_path();
        Self::from_path(path)
    }

    /// 全局包围盒（含自身 transform；`Group` 递归合并子项包围盒）。
    /// 路径取控制点包围盒（kurbo 语义，与 manim 控制点框一致，布局够用）。
    pub fn bbox(&self) -> Rect {
        let local = match &self.shape {
            Shape::Path(p) => {
                if p.elements().is_empty() {
                    Rect::ZERO
                } else {
                    p.bounding_box()
                }
            }
            Shape::Group(children) => children
                .iter()
                .map(Item::bbox)
                .reduce(|a, b| a.union(b))
                .unwrap_or(Rect::ZERO),
            Shape::Image(img) => Rect::new(0.0, 0.0, f64::from(img.width), f64::from(img.height)),
        };
        // 仿射变换四角后取轴对齐包围盒（含旋转的保守框）
        let t = self.transform;
        let xs = [
            (t * kurbo::Point::new(local.x0, local.y0)).x,
            (t * kurbo::Point::new(local.x1, local.y0)).x,
            (t * kurbo::Point::new(local.x0, local.y1)).x,
            (t * kurbo::Point::new(local.x1, local.y1)).x,
        ];
        let ys = [
            (t * kurbo::Point::new(local.x0, local.y0)).y,
            (t * kurbo::Point::new(local.x1, local.y0)).y,
            (t * kurbo::Point::new(local.x0, local.y1)).y,
            (t * kurbo::Point::new(local.x1, local.y1)).y,
        ];
        Rect::new(
            xs.iter().cloned().fold(f64::INFINITY, f64::min),
            ys.iter().cloned().fold(f64::INFINITY, f64::min),
            xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        )
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

    /// 设置对象 id（构建器风格）。
    pub fn with_id(mut self, id: ItemId) -> Self {
        self.id = id;
        self
    }

    /// 设置填充色（构建器风格）。
    pub fn with_fill(mut self, color: Color) -> Self {
        self.paint.fill = Some(color);
        self
    }

    /// 设置描边色（构建器风格）。
    pub fn with_stroke(mut self, color: Color) -> Self {
        self.paint.stroke = Some(color);
        self
    }

    /// 设置描边宽度（构建器风格）。
    pub fn with_stroke_width(mut self, width: f64) -> Self {
        self.paint.stroke_width = width;
        self
    }

    /// 设置仿射变换（构建器风格）。
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
    /// 画布宽（像素）。
    pub width: u32,
    /// 画布高（像素）。
    pub height: u32,
    /// 背景色。
    pub background: Color,
    /// 本帧全部可见对象（已按 z 序排列）。
    pub items: Vec<Item>,
}

impl DrawList {
    /// 创建空绘制指令列表。
    pub fn new(width: u32, height: u32, background: Color) -> Self {
        Self {
            width,
            height,
            background,
            items: Vec::new(),
        }
    }

    /// 追加一个对象（追加顺序即 z 序，后画在上）。
    pub fn push(&mut self, item: Item) {
        self.items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_of(item: &Item) -> &BezPath {
        match &item.shape {
            Shape::Path(p) => p,
            other => panic!("应为路径，得 {other:?}"),
        }
    }

    #[test]
    fn 正多边形闭合路径() {
        let item = Item::regular_polygon((100.0, 100.0).into(), 50.0, 3);
        let elements: Vec<_> = path_of(&item).elements().to_vec();
        // 3 顶点：move_to + 2 line_to + close = 4
        assert_eq!(elements.len(), 4);
        assert!(matches!(elements[0], kurbo::PathEl::MoveTo(_)));
    }

    #[test]
    fn 线段与箭头路径结构() {
        let line = Item::line((0.0, 0.0), (100.0, 0.0));
        assert_eq!(path_of(&line).elements().len(), 2, "move_to + line_to");

        // 箭头：线段（2 元素）+ 三角头（move + 2 line + close = 4）= 6
        let arrow = Item::arrow((0.0, 0.0), (100.0, 0.0));
        let els = path_of(&arrow).elements();
        assert_eq!(els.len(), 6);
        assert!(
            matches!(els[2], kurbo::PathEl::MoveTo(_)),
            "第 3 元素应起头部子路径"
        );
        assert!(matches!(els[5], kurbo::PathEl::ClosePath), "头部应闭合");
        // 线段止于头部底边：第 2 点 x < 终点 x
        if let kurbo::PathEl::LineTo(p) = els[1] {
            assert!(p.x < 100.0, "线段不应穿出箭头底部");
        } else {
            panic!("第 2 元素应为 LineTo");
        }
    }

    #[test]
    fn 虚线段数() {
        // 长 100、dash 10/gap 10 → 5 段虚线（每段 move+line = 2 元素）
        let d = Item::dashed_line((0.0, 0.0), (100.0, 0.0), 10.0, 10.0);
        assert_eq!(path_of(&d).elements().len(), 10);
        // 尾部不足时截断：长 25 → dash 10 + 5（截断），共 2 段
        let d2 = Item::dashed_line((0.0, 0.0), (25.0, 0.0), 10.0, 10.0);
        assert_eq!(path_of(&d2).elements().len(), 4);
    }

    #[test]
    fn 圆弧视觉逆时针包围盒() {
        // 从 0° 到 90° 视觉逆时针：屏幕 y 向下 → 弧在右上，y 为负
        let a = Item::arc((0.0, 0.0), 100.0, 0.0, 90.0);
        let bb = a.bbox();
        assert!((bb.x1 - 100.0).abs() < 1.0, "右边界 ≈100: {bb:?}");
        assert!(
            (bb.y0 + 100.0).abs() < 1.0,
            "上边界 ≈-100（视觉向上）: {bb:?}"
        );
        assert!(
            bb.y1 < 1.0 && bb.x0 > -1.0,
            "弧不应越出第一视觉象限: {bb:?}"
        );
    }

    #[test]
    fn 椭圆包围盒() {
        let e = Item::ellipse((50.0, 50.0), 30.0, 10.0);
        let bb = e.bbox();
        assert!(
            (bb.x0 - 20.0).abs() < 1.0 && (bb.x1 - 80.0).abs() < 1.0,
            "{bb:?}"
        );
        assert!(
            (bb.y0 - 40.0).abs() < 1.0 && (bb.y1 - 60.0).abs() < 1.0,
            "{bb:?}"
        );
    }

    #[test]
    fn 包围盒含变换与组合() {
        let r = Item::rect(Rect::new(0.0, 0.0, 100.0, 100.0)).with_translate(10.0, 20.0);
        let bb = r.bbox();
        assert_eq!(bb, Rect::new(10.0, 20.0, 110.0, 120.0), "平移后包围盒");

        let g = Item {
            id: 1,
            shape: Shape::Group(vec![
                Item::circle(Circle::new((0.0, 0.0), 10.0)),
                Item::circle(Circle::new((100.0, 0.0), 10.0)),
            ]),
            paint: Paint::default(),
            transform: Affine::IDENTITY,
        };
        let bb = g.bbox();
        assert!(
            (bb.x0 + 10.0).abs() < 1.0 && (bb.x1 - 110.0).abs() < 1.0,
            "组合取并集: {bb:?}"
        );
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

    #[test]
    fn 图片预乘转换() {
        let rgba = vec![255, 0, 0, 128, 0, 255, 0, 255];
        let asset = ImageAsset::from_rgba8(rgba, 2, 1).unwrap();
        // 半透明红：R 预乘为 128
        assert_eq!(&asset.rgba[0..4], &[128, 0, 0, 128]);
        // 全不透明绿：不变
        assert_eq!(&asset.rgba[4..8], &[0, 255, 0, 255]);
        // 长度不符报错
        assert!(ImageAsset::from_rgba8(vec![0; 3], 1, 1).is_err());
    }
}
