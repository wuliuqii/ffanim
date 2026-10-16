//! GPU 生产后端：Vello（wgpu compute shader）→ 纹理 → 读回像素。
//!
//! 初始化失败（无 GPU/驱动缺失）时返回错误，由调用方降级到 CpuBackend。
//!
//! vello 0.11 新架构：无状态机 API，fill/stroke 每次调用显式传变换与画笔。

use crate::{RenderBackend, WalkEvent, to_peniko, walk_events};
use ffanim_core::DrawList;

/// GPU 渲染后端（持有 wgpu 设备与 vello 渲染器）。
pub struct GpuBackend {
    device: wgpu::Device,
    queue: wgpu::Queue,
    renderer: vello::Renderer,
}

impl GpuBackend {
    /// 尝试初始化（同步阻塞；需要 GPU 或软件 Vulkan/OpenGL 实现）。
    pub fn new() -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|e| format!("未找到可用的 GPU 适配器: {e}"))?;
        // 埋点：适配器信息（info，排查驱动/后端选择问题用）
        let adapter_info = adapter.get_info();
        log::info!(
            "GPU 适配器: {}（{:?}，{:?}）",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device_type
        );

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("ffanim-gpu"),
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
        }))
        .map_err(|e| format!("GPU 设备创建失败: {e}"))?;

        let renderer = vello::Renderer::new(
            &device,
            vello::RendererOptions {
                use_cpu: false,
                antialiasing_support: vello::AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )
        .map_err(|e| format!("Vello 渲染器初始化失败: {e}"))?;

        Ok(Self {
            device,
            queue,
            renderer,
        })
    }

    /// 把 DrawList 编码为 vello Scene。
    fn encode(&self, draw: &DrawList) -> vello::Scene {
        let mut scene = vello::Scene::new();
        // 背景（vello 的 base_color 也会铺底，这里再显式铺一层保持与 CPU 后端一致）
        scene.fill(
            vello::peniko::Fill::NonZero,
            vello::kurbo::Affine::IDENTITY,
            &vello::peniko::Brush::from(to_peniko(draw.background)),
            None,
            &vello::kurbo::Rect::new(0.0, 0.0, f64::from(draw.width), f64::from(draw.height)),
        );

        walk_events(
            &draw.items,
            vello::kurbo::Affine::IDENTITY,
            &mut |ev| match ev {
                WalkEvent::Draw(t, item) => {
                    let paint = &item.paint;
                    match &item.shape {
                        ffanim_core::Shape::Image(img) => {
                            // Blob::new 需要 Arc<dyn AsRef<[u8]>>：unsize 强转要求源类型
                            // Sized，Arc<[u8]> 无法直接转，故拷贝进 Arc<Vec<u8>>
                            // （GPU 纹理上传本身也要拷贝，此处一次性 memcpy 可接受）
                            let data = vello::peniko::ImageData {
                                data: vello::peniko::Blob::new(std::sync::Arc::new(
                                    img.rgba.to_vec(),
                                )),
                                format: vello::peniko::ImageFormat::Rgba8,
                                alpha_type: vello::peniko::ImageAlphaType::AlphaPremultiplied,
                                width: img.width,
                                height: img.height,
                            };
                            let brush = vello::peniko::ImageBrush {
                                image: data,
                                sampler: Default::default(),
                            };
                            let alpha = paint.opacity as f32;
                            if alpha < 1.0 {
                                scene.push_layer(
                                    vello::peniko::Fill::NonZero,
                                    vello::peniko::Mix::Normal,
                                    alpha,
                                    vello::kurbo::Affine::IDENTITY,
                                    &vello::kurbo::Rect::new(
                                        0.0,
                                        0.0,
                                        f64::from(img.width),
                                        f64::from(img.height),
                                    ),
                                );
                            }
                            scene.draw_image(&brush, vello::kurbo::Affine::new(t.as_coeffs()));
                            if alpha < 1.0 {
                                scene.pop_layer();
                            }
                        }
                        ffanim_core::Shape::Path(path) => {
                            if let Some(fill) = paint.fill {
                                scene.fill(
                                    vello::peniko::Fill::NonZero,
                                    vello::kurbo::Affine::new(t.as_coeffs()),
                                    &vello::peniko::Brush::from(to_peniko(fill)),
                                    None,
                                    path,
                                );
                            }
                            if let Some(stroke) = paint.stroke {
                                scene.stroke(
                                    &vello::kurbo::Stroke::new(paint.stroke_width),
                                    vello::kurbo::Affine::new(t.as_coeffs()),
                                    &vello::peniko::Brush::from(to_peniko(stroke)),
                                    None,
                                    path,
                                );
                            }
                        }
                        ffanim_core::Shape::Group(_) => {} // Group 由 walk_events 展开
                    }
                }
                // 裁剪域：clip 路径为 Item 局部坐标，显式乘累积变换
                // （vello 层栈不保存变换，后续绘制变换不受影响）
                WalkEvent::PushClip(t, clip) => scene.push_clip_layer(
                    vello::peniko::Fill::NonZero,
                    vello::kurbo::Affine::new(t.as_coeffs()),
                    clip,
                ),
                WalkEvent::PopClip => scene.pop_layer(),
            },
        );
        scene
    }
}

impl RenderBackend for GpuBackend {
    fn render(&mut self, draw: &DrawList) -> Vec<u8> {
        let (w, h) = (draw.width, draw.height);
        let scene = self.encode(draw);

        // 离屏纹理
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ffanim-target"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        self.renderer
            .render_to_texture(
                &self.device,
                &self.queue,
                &scene,
                &texture.create_view(&wgpu::TextureViewDescriptor::default()),
                &vello::RenderParams {
                    base_color: to_peniko(draw.background),
                    width: w,
                    height: h,
                    antialiasing_method: vello::AaConfig::Area,
                },
            )
            .expect("vello 渲染失败");

        // 纹理 → 宿主缓冲（对齐 COPY_BYTES_PER_ROW_ALIGNMENT）
        let bytes_per_row = (w * 4).div_ceil(256) * 256;
        let buffer_size =
            wgpu::BufferSize::new(u64::from(bytes_per_row * h)).expect("缓冲区大小非法");
        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("ffanim-readback"),
            size: buffer_size.get(),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(h),
                },
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit([encoder.finish()]);
        let slice = buffer.slice(..);
        pollster::block_on(async {
            slice.map_async(wgpu::MapMode::Read, |r| r.expect("读回映射失败"));
            self.device
                .poll(wgpu::PollType::wait_indefinitely())
                .expect("设备轮询失败");
        });

        // 去除行对齐填充，得到紧凑 RGBA。注意双端输出约定不同（实证于
        // 透明通道验收，docs/07 §1.3）：vello（GPU）纹理输出已是 straight，
        // 直通即为约定语义；vello_cpu 输出 premultiplied，需在 CPU 路径
        // unpremultiply（lib.rs `unpremultiply`）——两端产物一致为 straight。
        let data = slice.get_mapped_range().expect("映射读取失败");
        let mut out = Vec::with_capacity((w * h * 4) as usize);
        for row in 0..h {
            let start = (row * bytes_per_row) as usize;
            out.extend_from_slice(&data[start..start + (w * 4) as usize]);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffanim_core::{Color, DrawList, Item};
    use kurbo::Circle;

    /// GPU 可用则验证输出，不可用则跳过（CI 无 GPU）。
    #[test]
    fn gpu出图冒烟() {
        let mut gpu = match GpuBackend::new() {
            Ok(g) => g,
            Err(_) => {
                eprintln!("跳过：无可用 GPU 适配器");
                return;
            }
        };
        let mut draw = DrawList::new(64, 64, Color::new(14, 17, 22, 255));
        draw.push(
            Item::circle(Circle::new((32.0, 32.0), 16.0)).with_fill(Color::new(255, 0, 0, 255)),
        );
        let buf = gpu.render(&draw);
        assert_eq!(buf.len(), 64 * 64 * 4);
        // 中心应为红色
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(
            buf[center] > 200,
            "GPU 中心应为红色: {:?}",
            &buf[center..center + 4]
        );
    }

    /// clip 层冒烟：矩形裁半（GPU 可用才验证，语义与 CPU 后端一致）。
    #[test]
    fn gpu裁剪冒烟() {
        let mut gpu = match GpuBackend::new() {
            Ok(g) => g,
            Err(_) => {
                eprintln!("跳过：无可用 GPU 适配器");
                return;
            }
        };
        let mut clip = kurbo::BezPath::new();
        clip.move_to((8.0, 8.0));
        clip.line_to((32.0, 8.0));
        clip.line_to((32.0, 56.0));
        clip.line_to((8.0, 56.0));
        clip.close_path();
        let mut draw = DrawList::new(64, 64, Color::new(0, 0, 0, 255));
        draw.push(
            Item::rect(kurbo::Rect::new(8.0, 8.0, 56.0, 56.0))
                .with_fill(Color::new(255, 0, 0, 255))
                .with_clip(clip),
        );
        let buf = gpu.render(&draw);
        let px = |x: usize, y: usize| buf[(y * 64 + x) * 4];
        assert!(
            px(16, 32) > 200,
            "clip 窗口内应为红: {:?}",
            &buf[(32 * 64 + 16) * 4..][..4]
        );
        assert_eq!(px(48, 32), 0, "clip 窗口外应为背景");
    }
}
