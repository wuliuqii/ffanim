//! M1 验收示例场景：三角形 + 文字 + 圆。
//!
//! 数据驱动：颜色全部来自主题，代码不出现硬编码颜色。

use ffanim::Theme;
use ffanim_core::kurbo::Shape as _;
use ffanim_core::kurbo::{Affine, Circle, Rect};
use ffanim_core::{DrawList, Font, Item};
use std::path::PathBuf;

/// 加载字体：显式路径优先，否则搜索系统常见位置（下沉为 core Font::from_system）。
pub fn load_font(explicit: Option<PathBuf>) -> ffanim::Result<Font> {
    Font::from_system(explicit)
}

/// 构建示例场景的 DrawList。
pub fn draw(width: u32, height: u32, theme: &Theme, font: &Font) -> DrawList {
    let mut draw = DrawList::new(width, height, theme.background().unwrap());

    let accent = theme.accent().unwrap();
    let foreground = theme.foreground().unwrap();
    let sw = f64::from(theme.style.stroke_width);

    // 三角形：左侧，描边样式
    let tri_center = (f64::from(width) * 0.25, f64::from(height) * 0.62);
    draw.push(
        Item::regular_polygon(tri_center.into(), 120.0, 3)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );

    // 圆：右侧，填充 + 描边
    let cir_center = (f64::from(width) * 0.75, f64::from(height) * 0.62);
    draw.push(
        Item::circle(Circle::new(cir_center, 110.0))
            .with_fill(accent)
            .with_stroke(foreground)
            .with_stroke_width(sw),
    );

    // 文字：顶部居中，轮廓提取后作为普通路径渲染
    let text_path = font.outline_text("ffanim M1", 72.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(width) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    draw.push(
        Item::from_path(text_path)
            .with_fill(foreground)
            .with_transform(Affine::translate((dx, f64::from(height) * 0.18))),
    );

    // 底部装饰矩形条（验证 Rect 构造与半透明填充）
    draw.push(
        Item::rect(Rect::new(
            0.0,
            f64::from(height) - 8.0,
            f64::from(width),
            f64::from(height),
        ))
        .with_fill(accent),
    );

    draw
}
