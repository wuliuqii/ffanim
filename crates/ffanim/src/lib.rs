//! ffanim：门面 crate，re-export 核心类型与各子系统，构成用户面向的 API。
//!
//! 用户只依赖 `ffanim`，不直接依赖内部 crate。
//! 最小出片路径：[`Timeline`] 描述动画 → [`render_to_mp4`] 一步出片。

pub use ffanim_core::kurbo;
pub use ffanim_core::{
    Anim, AnimKind, Axes, Camera, Color, CompiledTimeline, DrawList, Error, Font, ImageAsset, Item,
    RateFunc, Result, Style, Theme, Timeline, from_svg_file, from_svg_str, render_frame,
};

/// 公式/富文本排版子系统（Typst 嵌入）：TypstEngine / Formula / Text / match_formulas。
pub mod typst {
    pub use ffanim_typst::{
        Formula, FormulaOptions, TextOptions, TypstEngine, find_cjk_font, match_formulas,
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
    use ffanim_render::RenderBackend;
    let out = out.as_ref();
    // CpuBackend 无状态，每帧独立实例（闭包 Send+Sync，分段并行无需加锁）
    let frame_of = |f: usize| -> Vec<u8> {
        let draw = ffanim_core::render_frame(tl, f);
        ffanim_render::CpuBackend::new().render(&draw)
    };
    ffanim_encode::encode_video(
        tl.width,
        tl.height,
        tl.fps,
        tl.duration_frames,
        out,
        &ffanim_encode::EncodeOptions::default(),
        &frame_of,
    )
}
