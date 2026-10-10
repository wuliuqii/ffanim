//! ffanim-render：`DrawList → 像素`，GPU（Vello/wgpu）与 CPU（vello_cpu）双后端。
//!
//! 设计约束（docs/02-架构设计.md §3.4）：
//! - 两个后端消费同一 `DrawList`，单一路径表示，文本/公式已预提取为轮廓；
//! - CPU 后端是兜底路径（CI/无 GPU），GPU 后端是生产路径。

pub mod cpu;

#[cfg(feature = "gpu")]
pub mod gpu;

pub use cpu::CpuBackend;
#[cfg(feature = "gpu")]
pub use gpu::GpuBackend;

use ffanim_core::{Color, DrawList, Item, Shape};

/// 渲染后端抽象：DrawList → 非预乘 RGBA8 像素缓冲。
pub trait RenderBackend {
    fn render(&mut self, draw: &DrawList) -> Vec<u8>;
}

/// 每帧场景编码的公共遍历：按深度优先顺序产出 (累积变换, Item)。
pub fn walk_items<'a>(
    items: &'a [Item],
    parent: kurbo::Affine,
    f: &mut impl FnMut(kurbo::Affine, &'a Item),
) {
    for item in items {
        let transform = parent * item.transform;
        f(transform, item);
        if let Shape::Group(children) = &item.shape {
            walk_items(children, transform, f);
        }
    }
}

/// 颜色转换：core Color → peniko 色彩类型（各后端从自己的依赖树取 peniko，
/// 版本由各自 re-export 保证一致）。
pub fn to_peniko(color: Color) -> vello_cpu::peniko::Color {
    vello_cpu::peniko::Color::new([
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ])
}

/// 渲染单个 Item 到 CPU 上下文（填充 + 描边两遍）。
fn render_cpu_item(ctx: &mut vello_cpu::RenderContext, transform: kurbo::Affine, item: &Item) {
    let Shape::Path(path) = &item.shape else {
        return; // Group 由 walk_items 展开
    };
    ctx.set_transform(vello_cpu::kurbo::Affine::new(transform.as_coeffs()));
    let paint = &item.paint;

    if let Some(fill) = paint.fill {
        ctx.set_paint(to_peniko(fill));
        ctx.fill_path(path);
    }
    // 注意：stroke_path 使用当前 paint，描边前必须显式 set_paint(描边色)
    if let Some(stroke) = paint.stroke {
        ctx.set_paint(to_peniko(stroke));
        ctx.set_stroke(vello_cpu::kurbo::Stroke::new(paint.stroke_width));
        ctx.stroke_path(path);
    }
}

/// 非预乘 RGBA8 → PNG 文件。
pub fn save_png(
    path: impl AsRef<std::path::Path>,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> std::io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    image::save_buffer(path, rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| std::io::Error::other(e.to_string()))
}

/// kurbo 版本桥接：core 的 kurbo 0.13 与 vello_cpu 的 re-export 同版本，按系数重建。
pub(crate) fn to_kurbo(a: kurbo::Affine) -> vello_cpu::kurbo::Affine {
    vello_cpu::kurbo::Affine::new(a.as_coeffs())
}
