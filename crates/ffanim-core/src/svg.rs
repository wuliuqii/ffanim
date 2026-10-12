//! SVG 导入：usvg 解析 → tiny-skia 路径段 → kurbo → `Shape::Group`。
//!
//! 已知限制（warn 提示，不阻断渲染）：
//! - 渐变/图案填充与描边跳过（渲染层仅支持纯色）；
//! - `EvenOdd` 填充规则按 NonZero 渲染；
//! - 文本/图片节点跳过（usvg 未启用 text feature）。

use crate::item::{Item, Paint, Shape};
use crate::{Color, Error, Result};
use kurbo::{Affine, BezPath};
// kurbo::Shape 与本模块 Shape 枚举重名，匿名导入（bounding_box 等方法需要）
use tiny_skia_path::PathSegment;

/// 解析 SVG 字符串为组合对象（不支持的绘制以 warn 跳过）。
pub fn from_svg_str(svg: &str) -> Result<Item> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default())
        .map_err(|e| Error::Other(format!("SVG 解析失败: {e}")))?;
    convert_group(tree.root())
}

/// 从文件读取并解析 SVG。
pub fn from_svg_file(path: impl AsRef<std::path::Path>) -> Result<Item> {
    let path = path.as_ref();
    let svg = std::fs::read_to_string(path)
        .map_err(|e| Error::Other(format!("SVG 文件读取失败 {}: {e}", path.display())))?;
    from_svg_str(&svg)
}

/// 递归转换 Group：子节点自带绝对变换（abs_transform），无需手动累积。
fn convert_group(group: &usvg::Group) -> Result<Item> {
    let mut children = Vec::new();
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => children.push(convert_group(child)?),
            usvg::Node::Path(path) => {
                if let Some(item) = convert_path(path) {
                    children.push(item);
                }
            }
            other => log::warn!("SVG 导入跳过不支持的节点: {}", node_kind(other)),
        }
    }
    Ok(Item {
        id: 0,
        shape: Shape::Group(children),
        paint: Paint::default(),
        transform: Affine::IDENTITY,
    })
}

/// 转换单个路径节点（无填充无描边 = 不可见，返回 None 跳过）。
fn convert_path(node: &usvg::Path) -> Option<Item> {
    if node.fill().is_none() && node.stroke().is_none() {
        return None;
    }
    let mut bez = BezPath::new();
    for seg in node.data().segments() {
        let f = |v: f32| f64::from(v);
        match seg {
            PathSegment::MoveTo(p) => bez.move_to((f(p.x), f(p.y))),
            PathSegment::LineTo(p) => bez.line_to((f(p.x), f(p.y))),
            PathSegment::QuadTo(c, to) => bez.quad_to((f(c.x), f(c.y)), (f(to.x), f(to.y))),
            PathSegment::CubicTo(c0, c1, to) => {
                bez.curve_to((f(c0.x), f(c0.y)), (f(c1.x), f(c1.y)), (f(to.x), f(to.y)));
            }
            PathSegment::Close => bez.close_path(),
        }
    }
    // tiny-skia Transform → kurbo Affine：系数行序差异，ky/kx 互换（与 ffanim-typst 映射一致）
    let t = node.abs_transform();
    let transform = Affine::new([
        f64::from(t.sx),
        f64::from(t.ky),
        f64::from(t.kx),
        f64::from(t.sy),
        f64::from(t.tx),
        f64::from(t.ty),
    ]);
    let mut item = Item::from_path(bez).with_transform(transform);
    if let Some(fill) = node.fill() {
        if let Some(color) = paint_color(fill.paint(), f64::from(fill.opacity().get())) {
            item = item.with_fill(color);
        } else {
            log::warn!("SVG 填充为渐变/图案，已忽略填充");
        }
        if fill.rule() == usvg::FillRule::EvenOdd {
            log::warn!("SVG 填充规则 EvenOdd 暂不支持，按 NonZero 渲染");
        }
    }
    if let Some(stroke) = node.stroke() {
        match paint_color(stroke.paint(), f64::from(stroke.opacity().get())) {
            Some(color) => {
                item = item
                    .with_stroke(color)
                    .with_stroke_width(f64::from(stroke.width().get()));
            }
            None => log::warn!("SVG 描边为渐变/图案，已忽略描边"),
        }
    }
    Some(item)
}

/// 纯色画笔 → 颜色（渐变/图案返回 None）。
fn paint_color(paint: &usvg::Paint, opacity: f64) -> Option<Color> {
    match paint {
        usvg::Paint::Color(c) => {
            let a = (opacity * 255.0).round() as u8;
            Some(Color::new(c.red, c.green, c.blue, a))
        }
        _ => None,
    }
}

fn node_kind(node: &usvg::Node) -> &'static str {
    match node {
        usvg::Node::Group(_) => "group",
        usvg::Node::Path(_) => "path",
        usvg::Node::Image(_) => "image",
        usvg::Node::Text(_) => "text",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Shape as _;

    #[test]
    fn 解析简单矩形() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="80">
            <rect x="10" y="20" width="30" height="40" fill="#ff0000"/>
        </svg>"##;
        let item = from_svg_str(svg).unwrap();
        let Shape::Group(children) = &item.shape else {
            panic!("应为组合");
        };
        assert_eq!(children.len(), 1);
        let Shape::Path(path) = &children[0].shape else {
            panic!("应为路径");
        };
        let b = path.bounding_box();
        assert!((b.x0 - 10.0).abs() < 1e-6);
        assert!((b.y0 - 20.0).abs() < 1e-6);
        assert_eq!(children[0].paint.fill, Some(Color::new(255, 0, 0, 255)));
    }

    #[test]
    fn 渐变填充跳过() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="50">
            <defs>
                <linearGradient id="g">
                    <stop offset="0" stop-color="#ffffff"/>
                    <stop offset="1" stop-color="#000000"/>
                </linearGradient>
            </defs>
            <rect x="0" y="0" width="10" height="10" fill="url(#g)" stroke="#00ff00"/>
        </svg>"##;
        let item = from_svg_str(svg).unwrap();
        let Shape::Group(children) = &item.shape else {
            panic!("应为组合");
        };
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].paint.fill, None);
        assert_eq!(children[0].paint.stroke, Some(Color::new(0, 255, 0, 255)));
    }

    #[test]
    fn 组变换提升到元素上() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <g transform="translate(5 7)">
                <circle cx="10" cy="10" r="5" fill="#0000ff"/>
            </g>
        </svg>"##;
        let item = from_svg_str(svg).unwrap();
        let Shape::Group(children) = &item.shape else {
            panic!("应为组合");
        };
        // <g> 是 root 下的嵌套组合
        assert_eq!(children.len(), 1);
        let Shape::Group(inner) = &children[0].shape else {
            panic!("应为嵌套组合");
        };
        // 组变换以 abs_transform 形式挂在子路径上
        assert_eq!(
            inner[0].transform,
            Affine::new([1.0, 0.0, 0.0, 1.0, 5.0, 7.0])
        );
    }
}
