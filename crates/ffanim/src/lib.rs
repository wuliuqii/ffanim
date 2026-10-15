//! ffanim：门面 crate，re-export 核心类型与各子系统，构成用户面向的 API。
//!
//! 用户只依赖 `ffanim`，不直接依赖内部 crate。
//! 最小出片路径：[`Timeline`] 描述动画 → [`render_to_mp4`] 一步出片。

pub use ffanim_core::kurbo;
pub use ffanim_core::{
    Anim, AnimKind, Axes, BoolOp, Camera, Color, CompiledTimeline, Direction, DrawList, Error,
    Font, ImageAsset, Item, ItemId, MatchPlan, PoseDelta, RateFunc, RateTable, Result, Style,
    TableOptions, TableSpec, Theme, Timeline, arrange, from_svg_file, from_svg_str, match_shapes,
    render_frame, table,
};
pub use ffanim_render::FrameBackend;

/// 场景创作糖层（docs/07 评审 D1/D3）：Stage 工作台 + 像素意图落位。
///
/// 装配四件套（Theme/引擎/字体/时间轴）与文本上屏四拍（排版→拆件→落位→
/// 编排）一步到位；纯组合器，可随时经 `tl()`/`into_timeline()` 绕过回原语层。
/// 落位锚点 [`Anchor`](ffanim_typst::Anchor) 从 [`typst`] 模块引入。
pub mod stage;

/// 公式/富文本排版子系统（Typst 嵌入）：TypstEngine / Formula / Text / match_formulas。
pub mod typst {
    pub use ffanim_typst::{
        Anchor, ArrowKind, CodeOptions, CodeTheme, DecimalNumberOptions, Delimiter, Formula,
        FormulaOptions, TextOptions, TypstEngine, find_cjk_font, match_formulas,
    };
}

/// 音频子系统：analyze 诊断 / mux 音轨混流。
pub mod audio {
    pub use ffanim_audio::{AudioReport, analyze, mux};
}

/// Agent 视觉诊断：strip 接触表 / onion 洋葱皮（与 CLI 共用同一实现）。
pub mod tools {
    pub use ffanim_render::tools::{
        OnionReport, StripReport, downsample, onion, sample_frames, strip,
    };
}

/// 项目版本（供 CLI 与元信息使用）。
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 一键出片：CPU 渲染 + ffmpeg 管道编码（无 GPU 依赖，效果与预览一致）。
///
/// # 示例
///
/// ```no_run
/// use ffanim::{Anim, Color, Item, Timeline};
///
/// let mut tl = Timeline::new(960, 540, 30, Color::new(14, 17, 22, 255));
/// let cir = tl.add(Item::circle(ffanim::kurbo::Circle::new((480.0, 270.0), 80.0)));
/// tl.play(Anim::create(cir).during(1.0));
/// ffanim::render_to_mp4(&tl.compile(), "out/demo.mp4").unwrap();
/// ```
pub fn render_to_mp4(tl: &CompiledTimeline, out: impl AsRef<std::path::Path>) -> Result<()> {
    render_to_mp4_frames(tl, out, 0..tl.duration_frames)
}

/// 一键出片（片段导出）：只编码 `frames` 帧区间（半开），帧内容取自全片时间轴。
///
/// 帧号平移：区间内第 f 帧 = 全片第 `frames.start + f` 帧（渲染是纯函数查表），
/// 复用同一条 ffmpeg 管道。用于 `--section` 片段导出与分段出片。
///
/// 渲染后端：自动选择（GPU 优先，无 GPU/驱动缺失自动降级 CPU，见
/// [`FrameBackend::auto`]）。需要确定性输出（回归比对）时用
/// [`render_to_mp4_cpu`]。
///
/// # 示例
///
/// ```no_run
/// use ffanim::{Anim, Color, Item, Timeline};
///
/// let mut tl = Timeline::new(960, 540, 30, Color::new(14, 17, 22, 255));
/// let cir = tl.add(Item::circle(ffanim::kurbo::Circle::new((480.0, 270.0), 80.0)));
/// tl.play(Anim::create(cir).during(1.0));
/// // 只导出第 15..45 帧（[0.5s, 1.5s)）
/// ffanim::render_to_mp4_frames(&tl.compile(), "out/clip.mp4", 15..45).unwrap();
/// ```
///
/// # Errors
/// 区间为空或越界（`end > 全片帧数`）时返回 [`Error::Other`]。
pub fn render_to_mp4_frames(
    tl: &CompiledTimeline,
    out: impl AsRef<std::path::Path>,
    frames: std::ops::Range<usize>,
) -> Result<()> {
    render_to_mp4_frames_with(tl, out, frames, ffanim_render::FrameBackend::auto)
}

/// 一键出片（强制 CPU）：与 [`render_to_mp4`] 同一管线，但帧渲染锁定
/// vello_cpu 后端——逐字节确定，供回归比对 / CI / 无 GPU 环境显式选择。
pub fn render_to_mp4_cpu(tl: &CompiledTimeline, out: impl AsRef<std::path::Path>) -> Result<()> {
    render_to_mp4_frames_with(tl, out, 0..tl.duration_frames, || {
        ffanim_render::FrameBackend::cpu()
    })
}

/// PNG 序列帧导出：`{dir}/{n:04}.png`（0000 起，全片）——下游剪辑/审片的
/// 标准中间格式、论文插图逐帧提取。渲染走与出片相同的后端自动选择
/// （GPU 优先）；PNG 编码纯 Rust（不走 ffmpeg），worker 数为核数一半。
///
/// # Errors
/// 目录创建/文件写出失败时返回 [`Error::Other`]。
pub fn render_to_png_frames(tl: &CompiledTimeline, dir: impl AsRef<std::path::Path>) -> Result<()> {
    render_to_png_frames_range(tl, dir, 0..tl.duration_frames)
}

/// PNG 序列帧导出（片段）：只写 `frames` 帧区间（半开），帧内容取自全片
/// 时间轴，文件名仍为全片帧号（`{dir}/{first+f:04}.png`），便于与 mp4 对帧。
pub fn render_to_png_frames_range(
    tl: &CompiledTimeline,
    dir: impl AsRef<std::path::Path>,
    frames: std::ops::Range<usize>,
) -> Result<()> {
    if frames.is_empty() || frames.end > tl.duration_frames {
        return Err(Error::Other(format!(
            "帧区间 {frames:?} 越界：全片共 {} 帧（{:.2}s）",
            tl.duration_frames,
            tl.duration_frames as f64 / f64::from(tl.fps),
        )));
    }
    use ffanim_render::RenderBackend as _;
    use std::sync::Mutex;
    let backend = Mutex::new(ffanim_render::FrameBackend::auto());
    let dir = dir.as_ref();
    let first = frames.start;
    ffanim_render::write_png_sequence(dir, tl.width, tl.height, first, frames.len(), &|f| {
        let draw = ffanim_core::render_frame(tl, first + f);
        backend.lock().unwrap().render(&draw)
    })
    .map_err(|e| Error::Other(format!("PNG 序列写出失败: {e}")))
}

/// 指定后端构造器的片段出片（`make_backend` 在编码启动前调用一次，
/// 同一实例被并行帧闭包经 `Mutex` 共享）。
fn render_to_mp4_frames_with(
    tl: &CompiledTimeline,
    out: impl AsRef<std::path::Path>,
    frames: std::ops::Range<usize>,
    make_backend: impl FnOnce() -> ffanim_render::FrameBackend,
) -> Result<()> {
    if frames.is_empty() || frames.end > tl.duration_frames {
        return Err(Error::Other(format!(
            "帧区间 {frames:?} 越界：全片共 {} 帧（{:.2}s）",
            tl.duration_frames,
            tl.duration_frames as f64 / f64::from(tl.fps),
        )));
    }
    use std::sync::Mutex;
    // render 是 trait 方法，需将 trait 引入作用域
    use ffanim_render::RenderBackend;
    let out = out.as_ref();
    let first = frames.start;
    let total = frames.len();
    // 后端单实例 + Mutex 共享（vello Renderer/wgpu 资源均为 Send）：
    // GPU 提交串行化，帧求值（DrawList 查表）仍然分段并行
    let backend = Mutex::new(make_backend());
    println!("渲染后端: {}", backend.lock().unwrap().name());
    let frame_of = move |f: usize| -> Vec<u8> {
        let draw = ffanim_core::render_frame(tl, first + f);
        backend.lock().unwrap().render(&draw)
    };
    ffanim_encode::encode_video(
        tl.width,
        tl.height,
        tl.fps,
        total,
        out,
        &ffanim_encode::EncodeOptions::default(),
        &frame_of,
    )
}
