//! ffanim-render：`DrawList → 像素`，GPU（Vello/wgpu）与 CPU（vello_cpu）双后端。
//!
//! 设计约束（docs/02-架构设计.md §3.4）：
//! - 两个后端消费同一 `DrawList`，单一路径表示，文本/公式已预提取为轮廓；
//! - CPU 后端是兜底路径（CI/无 GPU），GPU 后端是生产路径。

pub mod backend;
pub mod cpu;
pub mod tools;

#[cfg(feature = "gpu")]
pub mod gpu;

pub use backend::FrameBackend;
pub use cpu::CpuBackend;
#[cfg(feature = "gpu")]
pub use gpu::GpuBackend;

use ffanim_core::{Color, DrawList, Item, Shape};

/// 渲染后端抽象：DrawList → 非预乘 RGBA8 像素缓冲。
pub trait RenderBackend {
    /// 渲染一帧：DrawList → RGBA8 像素缓冲（非预乘，行优先）。
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

/// 渲染单个 Item 到 CPU 上下文（填充 + 描边两遍；图片走 fill_rect + 透明度层）。
fn render_cpu_item(ctx: &mut vello_cpu::RenderContext, transform: kurbo::Affine, item: &Item) {
    ctx.set_transform(vello_cpu::kurbo::Affine::new(transform.as_coeffs()));
    match &item.shape {
        Shape::Path(path) => {
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
        Shape::Image(img) => {
            let (Ok(w), Ok(h)) = (u16::try_from(img.width), u16::try_from(img.height)) else {
                log::warn!(
                    "图片尺寸超 CPU 后端上限（65535px），跳过: {}x{}",
                    img.width,
                    img.height
                );
                return;
            };
            // from_parts 消费 Vec<u8>（bytemuck 转为 PremulRgba8），故逐帧拷贝一份
            let pixmap = vello_cpu::Pixmap::from_parts(
                img.rgba.to_vec(),
                w,
                h,
                vello_cpu::PixelMetadata::new(
                    vello_cpu::peniko::ImageAlphaType::AlphaPremultiplied,
                    true,
                ),
            );
            let image = vello_cpu::Image {
                image: vello_cpu::ImageSource::Pixmap(std::sync::Arc::new(pixmap)),
                sampler: Default::default(),
            };
            let opacity = item.paint.opacity as f32;
            if opacity < 1.0 {
                ctx.push_opacity_layer(opacity);
            }
            ctx.set_paint(image);
            ctx.fill_rect(&vello_cpu::kurbo::Rect::new(
                0.0,
                0.0,
                f64::from(img.width),
                f64::from(img.height),
            ));
            if opacity < 1.0 {
                ctx.pop_layer();
            }
        }
        Shape::Group(_) => {} // Group 由 walk_items 展开
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

/// PNG 文件 → 非预乘 RGBA8（snapshot 基准图读取用）。
pub fn load_png(path: impl AsRef<std::path::Path>) -> std::io::Result<(u32, u32, Vec<u8>)> {
    let img = image::open(path.as_ref())
        .map_err(|e| std::io::Error::other(e.to_string()))?
        .into_rgba8();
    let (width, height) = img.dimensions();
    Ok((width, height, img.into_raw()))
}

/// 帧序列并行写为 PNG（`{dir}/{start+n:04}.png`）——下游剪辑/审片的
/// 标准中间格式。`frames(i)` 产出第 `start+i` 帧像素（通常 = 后端渲染 +
/// 查表求值）；片段导出时传全片帧号偏移可保持文件名与 mp4 对帧。
/// 原子计数器动态分派帧号（首帧排版冷启动慢，静态均分易负载倾斜），
/// worker 数为核数一半（与分段编码同策略，不超订阅）。
pub fn write_png_sequence(
    dir: impl AsRef<std::path::Path>,
    width: u32,
    height: u32,
    start: usize,
    total: usize,
    frames: &(impl Fn(usize) -> Vec<u8> + Sync),
) -> std::io::Result<()> {
    use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
    let dir = dir.as_ref();
    std::fs::create_dir_all(dir)?;
    let workers = std::thread::available_parallelism()
        .map(|n| (n.get() / 2).max(1))
        .unwrap_or(1)
        .min(total.max(1));
    let next = AtomicUsize::new(0);
    let first_err: std::sync::Mutex<Option<std::io::Error>> = std::sync::Mutex::new(None);
    std::thread::scope(|s| {
        for _ in 0..workers {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Relaxed);
                    if i >= total {
                        break;
                    }
                    let rgba = frames(i);
                    debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
                    let path = dir.join(format!("{:04}.png", start + i));
                    if let Err(e) = save_png(&path, width, height, &rgba) {
                        first_err.lock().unwrap().replace(e);
                        break;
                    }
                }
            });
        }
    });
    match first_err.into_inner().unwrap() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

/// kurbo 版本桥接：core 的 kurbo 0.13 与 vello_cpu 的 re-export 同版本，按系数重建。
pub(crate) fn to_kurbo(a: kurbo::Affine) -> vello_cpu::kurbo::Affine {
    vello_cpu::kurbo::Affine::new(a.as_coeffs())
}
