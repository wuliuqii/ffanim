//! 渲染族命令：render（mp4 出片）/ timeline（调度表检查）/ frame（单帧 PNG）。

use crate::{demo, scenes};
use clap::Args;
use std::path::PathBuf;
use std::time::Instant;

/// 渲染场景为 mp4 视频
#[derive(Args)]
pub struct RenderArgs {
    /// 场景名（见 scenes.rs 注册表）
    #[arg(long, short)]
    pub scene: String,

    /// 输出 mp4 路径
    #[arg(long, short, default_value = "out/demo.mp4")]
    pub out: PathBuf,

    /// 帧率
    #[arg(long, default_value_t = 30)]
    pub fps: u32,

    /// 并行分段数（默认 = CPU 核数）
    #[arg(long)]
    pub segments: Option<usize>,

    /// 音轨文件（渲染后混入；片长延伸到音频长度，Duration::FromAudio 语义）
    #[arg(long)]
    pub audio: Option<PathBuf>,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 打印编译后时间轴（动画调度表，供 agent 检查）
#[derive(Args)]
pub struct TimelineArgs {
    /// 场景名
    #[arg(long, short)]
    pub scene: String,

    /// 以 JSON 输出
    #[arg(long)]
    pub json: bool,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 渲染单帧示例场景为 PNG（M1 验收场景：三角形 + 文字 + 圆）
#[derive(Args)]
pub struct FrameArgs {
    /// 输出 PNG 路径
    #[arg(long, short, default_value = "frames/demo.png")]
    pub out: PathBuf,

    /// 画布宽度（px）
    #[arg(long, default_value_t = 960)]
    pub width: u32,

    /// 画布高度（px）
    #[arg(long, default_value_t = 540)]
    pub height: u32,

    /// 渲染后端（cpu / gpu，gpu 需编译时启用 gpu 特性）
    #[arg(long, default_value = "cpu")]
    pub backend: String,

    /// 字体文件路径（缺省时搜索系统常见位置）
    #[arg(long, short)]
    pub font: Option<PathBuf>,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 渲染场景为 mp4。
pub fn render(args: RenderArgs) -> ffanim::Result<()> {
    let RenderArgs {
        scene,
        out,
        fps,
        segments,
        audio,
        theme,
    } = args;
    let theme = ffanim::Theme::from_path(&theme)?;
    let mut tl = scenes::build(&scene, &theme, fps)?.compile();

    // Duration::FromAudio：片长至少延伸到音频长度（尾帧定格）
    let audio_tmp: Option<PathBuf> = match &audio {
        Some(a) => {
            let report = ffanim_audio::analyze(a)?;
            let frames = (report.duration * f64::from(fps)).ceil() as usize;
            println!(
                "音轨 {a:?}: {:.2}s → 片长延伸到 {frames} 帧",
                report.duration
            );
            tl = tl.with_min_duration(frames);
            Some(out.with_extension("mux.mp4"))
        }
        None => None,
    };

    println!(
        "场景 {scene}: {} 帧 / {:.2}s @{}fps，{} 个动画",
        tl.duration_frames,
        tl.duration_frames as f64 / f64::from(tl.fps),
        tl.fps,
        tl.anims.len()
    );

    use ffanim_render::RenderBackend;
    let (w, h, fps, total) = (tl.width, tl.height, tl.fps, tl.duration_frames);
    let video = total as f64 / f64::from(fps);
    let tl = std::sync::Arc::new(tl);
    // 取帧闭包：渲染是纯函数，多线程调用安全（每帧独立 CPU 上下文）
    let frame_of = move |f: usize| -> Vec<u8> {
        let draw = ffanim_core::render_frame(&tl, f);
        ffanim_render::CpuBackend::new().render(&draw)
    };

    let opts = ffanim_encode::EncodeOptions {
        segments,
        ..Default::default()
    };
    let start = Instant::now();
    // 有音轨时先渲染到临时文件，混流后删除
    let encode_out = audio_tmp.clone().unwrap_or_else(|| out.clone());
    ffanim_encode::encode_video(w, h, fps, total, &encode_out, &opts, &frame_of)?;
    let elapsed = start.elapsed().as_secs_f64();
    println!(
        "已输出 {encode_out:?}（渲染 {elapsed:.2}s / 视频时长 {video:.2}s，实时率 {:.1}x）",
        video / elapsed
    );

    // 音轨混流：视频流复制 + AAC，产物落最终 out
    if let (Some(a), Some(tmp)) = (&audio, &audio_tmp) {
        ffanim_audio::mux(tmp, a, &out)?;
        std::fs::remove_file(tmp).ok();
        println!("音轨已混入 {out:?}");
    }
    Ok(())
}

/// 打印编译后时间轴。
pub fn timeline(args: TimelineArgs) -> ffanim::Result<()> {
    let TimelineArgs { scene, json, theme } = args;
    let theme = ffanim::Theme::from_path(&theme)?;
    let tl = scenes::build(&scene, &theme, 30)?.compile();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "scene": scene,
                "fps": tl.fps,
                "duration_frames": tl.duration_frames,
                "duration_seconds": tl.duration_frames as f64 / f64::from(tl.fps),
                "items": tl.items.len(),
                "anims": tl.anims.iter().map(|a| ffanim_core::AnimKind::describe(&a.kind)).collect::<Vec<_>>(),
            })
        );
    } else {
        println!(
            "场景 {scene}: {} 帧 / {:.2}s @30fps",
            tl.duration_frames,
            tl.duration_frames as f64 / 30.0
        );
        for a in &tl.anims {
            println!(
                "  [{:>4}..{:<4}] {:>5.2}s..{:<5.2}s {}",
                a.start,
                a.end,
                a.start as f64 / 30.0,
                a.end as f64 / 30.0,
                ffanim_core::AnimKind::describe(&a.kind)
            );
        }
    }
    Ok(())
}

/// 渲染单帧示例场景为 PNG。
pub fn frame(args: FrameArgs) -> ffanim::Result<()> {
    let FrameArgs {
        out,
        width,
        height,
        backend,
        font,
        theme,
    } = args;
    let theme = ffanim::Theme::from_path(&theme)?;
    let font = demo::load_font(font)?;
    let draw = demo::draw(width, height, &theme, &font);

    let rgba = match backend.as_str() {
        "cpu" => {
            use ffanim_render::RenderBackend;
            let mut cpu = ffanim_render::CpuBackend::new();
            Ok(cpu.render(&draw))
        }
        #[cfg(feature = "gpu")]
        "gpu" => {
            use ffanim_render::RenderBackend;
            let mut gpu = ffanim_render::GpuBackend::new().map_err(ffanim::Error::Other)?;
            Ok(gpu.render(&draw))
        }
        other => Err(ffanim::Error::Other(format!(
            "未知后端 {other}（本二进制编译时可用的后端见 --help）"
        ))),
    }?;

    ffanim_render::save_png(&out, width, height, &rgba)
        .map_err(|e| ffanim::Error::ThemeIo(e, out.clone()))?;
    println!("已输出 {out:?}（{width}x{height}，{backend} 后端）");
    Ok(())
}
