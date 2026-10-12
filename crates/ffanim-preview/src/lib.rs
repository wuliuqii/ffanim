//! ffanim-preview：winit + softbuffer 实时预览（播放/暂停/逐帧/seek）。
//!
//! 设计要点：
//! - 渲染复用离线管线的 CPU 后端（与 mp4 输出像素级一致），softbuffer 直写窗口；
//! - 不引入 egui/wgpu：避免与 vello 的 wgpu 30 版本冲突，UI 需求极简（进度条 + 键盘）；
//! - 求值用 [`ffanim_core::render_frame`] 纯函数，seek 天然支持（跳帧求值无状态）。
//!
//! 交互：
//! - `空格` 播放/暂停；`←/→` ±1 帧；`Shift+←/→` ±10 帧；`Home/End` 首尾帧；
//! - 鼠标点击/拖动底部进度条 seek；`Q/Esc` 退出。

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ffanim_core::CompiledTimeline;
use ffanim_render::{CpuBackend, RenderBackend};
use softbuffer::Surface;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowId};

/// 进度条高度（px）。
const UI_HEIGHT: f64 = 40.0;

/// 运行预览窗口（阻塞至窗口关闭）。
pub fn run(tl: CompiledTimeline, accent: ffanim_core::Color) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("创建事件循环失败: {e}"))?;
    let mut app = PreviewApp {
        tl,
        accent,
        frame: 0,
        playing: false,
        backend: CpuBackend::new(),
        window: None,
        surface: None,
        modifiers: ModifiersState::empty(),
        cursor_x: 0.0,
        drag_seek: false,
        last_frame_at: Instant::now(),
    };
    event_loop
        .run_app(&mut app)
        .map_err(|e| format!("事件循环错误: {e}"))
}

struct PreviewApp {
    tl: CompiledTimeline,
    accent: ffanim_core::Color,
    frame: usize,
    playing: bool,
    backend: CpuBackend,
    window: Option<Arc<Window>>,
    /// softbuffer Surface（Context 生命周期包含在其内）。
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    modifiers: ModifiersState,
    cursor_x: f64,
    drag_seek: bool,
    last_frame_at: Instant,
}

impl PreviewApp {
    fn total(&self) -> usize {
        self.tl.duration_frames.saturating_sub(1)
    }

    fn set_frame(&mut self, frame: usize) {
        let new = frame.min(self.total());
        if new != self.frame {
            self.frame = new;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }

    fn draw(&mut self) {
        if self.window.is_none() || self.surface.is_none() {
            return;
        }
        let (w, h) = (self.tl.width, self.tl.height);
        let ui_h = UI_HEIGHT as u32;
        let win_w = w as usize;
        let win_h = h as usize + ui_h as usize;

        // 帧像素：与离线渲染完全一致的 CPU 后端（先完成渲染，避免与 surface 借用冲突）
        let draw = ffanim_core::render_frame(&self.tl, self.frame);
        let rgba = self.backend.render(&draw);
        let (accent, frame, total) = (self.accent, self.frame, self.total());

        let window = self.window.as_ref().unwrap();
        let surface = self.surface.as_mut().unwrap();
        // buffer 尺寸跟随窗口（resize 幂等，不可缩放窗口下仅首次生效）
        let (bw, bh) = (
            NonZeroU32::new(win_w as u32).expect("宽度非零"),
            NonZeroU32::new(win_h as u32).expect("高度非零"),
        );
        surface
            .resize(bw, bh)
            .map_err(|e| format!("调整绘制表面失败: {e}"))
            .expect("调整绘制表面失败");

        let mut buffer = surface
            .buffer_mut()
            .map_err(|e| format!("获取绘制缓冲失败: {e}"))
            .expect("获取绘制缓冲失败");
        {
            let pixels: &mut [u32] = &mut buffer;
            // 上半：帧内容 RGBA → xRGB
            for (i, px) in pixels[..win_w * (win_h - ui_h as usize)]
                .iter_mut()
                .enumerate()
            {
                let o = i * 4;
                *px = rgb_u32(rgba[o], rgba[o + 1], rgba[o + 2]);
            }
            // 下半：进度条 UI
            draw_ui(pixels, win_w, win_h, ui_h as usize, accent, frame, total);
        }
        buffer
            .present()
            .map_err(|e| format!("呈现失败: {e}"))
            .expect("呈现失败");

        // 窗口标题即状态行（免字体依赖）
        let sec = frame as f64 / f64::from(self.tl.fps);
        window.set_title(&format!(
            "ffanim  帧 {frame}/{total}  {sec:.2}s  [空格播放 ←/→逐帧 拖进度条 Q退出]"
        ));
    }

    fn seek_to(&mut self, x: f64) {
        let Some(w) = &self.window else {
            return;
        };
        let width = w.inner_size().width as f64;
        if width <= 0.0 {
            return;
        }
        let progress = (x / width).clamp(0.0, 1.0);
        self.playing = false;
        self.set_frame((progress * self.total() as f64).round() as usize);
    }
}

#[inline]
fn rgb_u32(r: u8, g: u8, b: u8) -> u32 {
    (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// 底部 UI：深灰底条 + accent 已播进度 + 亮色手柄。
fn draw_ui(
    pixels: &mut [u32],
    win_w: usize,
    win_h: usize,
    ui_h: usize,
    accent: ffanim_core::Color,
    frame: usize,
    total: usize,
) {
    let ui_start = win_h - ui_h;
    let bar_y0 = ui_start + (ui_h - 8) / 2;
    let bar_h = 8usize;
    let bg = rgb_u32(0x22, 0x26, 0x2e);
    let accent = rgb_u32(accent.r, accent.g, accent.b);
    let handle = rgb_u32(0xf0, 0xf0, 0xf0);
    let progress = if total == 0 {
        1.0
    } else {
        frame as f64 / total as f64
    };
    let filled_x = (progress * win_w as f64) as usize;
    for y in ui_start..win_h {
        for x in 0..win_w {
            let px = &mut pixels[y * win_w + x];
            let in_bar = y >= bar_y0 && y < bar_y0 + bar_h;
            *px = if !in_bar {
                bg
            } else if x <= filled_x {
                accent
            } else {
                bg
            };
        }
    }
    // 手柄（4px 宽竖条）
    let hx = (progress * (win_w - 4) as f64) as usize;
    for y in bar_y0.saturating_sub(4)..(bar_y0 + bar_h + 4).min(win_h) {
        for x in hx..(hx + 4).min(win_w) {
            pixels[y * win_w + x] = handle;
        }
    }
}

impl ApplicationHandler for PreviewApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let (w, h) = (self.tl.width, self.tl.height);
        let attrs = Window::default_attributes()
            .with_title("ffanim preview")
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(f64::from(w), f64::from(h) + UI_HEIGHT));
        let window = event_loop.create_window(attrs).expect("创建窗口失败");
        let window = Arc::new(window);
        let context = softbuffer::Context::new(Arc::clone(&window)).expect("创建绘制上下文失败");
        let surface =
            softbuffer::Surface::new(&context, Arc::clone(&window)).expect("创建绘制表面失败");
        // Surface 内聚平台状态（Box Dispatch），context 就地丢弃
        drop(context);
        self.window = Some(window);
        self.surface = Some(surface);
        self.window.as_ref().unwrap().request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.draw(),

            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state != ElementState::Pressed {
                    return;
                }
                let shift = self.modifiers.contains(ModifiersState::SHIFT);
                match event.logical_key {
                    Key::Named(NamedKey::Space) => {
                        self.playing = !self.playing;
                        self.last_frame_at = Instant::now();
                    }
                    Key::Named(NamedKey::ArrowLeft) => {
                        self.playing = false;
                        self.set_frame(self.frame.saturating_sub(if shift { 10 } else { 1 }));
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        self.playing = false;
                        self.set_frame(self.frame + if shift { 10 } else { 1 });
                    }
                    Key::Named(NamedKey::Home) => self.set_frame(0),
                    Key::Named(NamedKey::End) => self.set_frame(self.total()),
                    Key::Named(NamedKey::Escape) => event_loop.exit(),
                    Key::Character(c) if c.to_lowercase() == "q" => event_loop.exit(),
                    _ => {}
                }
            }

            // 进度条 seek：按下开始拖动，移动更新，松开结束
            WindowEvent::MouseInput { state, .. } => {
                self.drag_seek = state == ElementState::Pressed;
                if self.drag_seek {
                    self.seek_to(self.cursor_x);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x;
                if self.drag_seek {
                    self.seek_to(position.x);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if !self.playing {
            return;
        }
        let frame_dur = Duration::from_secs_f64(1.0 / f64::from(self.tl.fps));
        if self.last_frame_at.elapsed() >= frame_dur {
            self.last_frame_at = Instant::now();
            if self.frame >= self.total() {
                self.playing = false; // 播完暂停在尾帧
            } else {
                self.frame += 1;
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
        // 下一帧唤醒（避免忙轮询）
        event_loop.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + frame_dur.saturating_sub(self.last_frame_at.elapsed()),
        ));
    }
}
