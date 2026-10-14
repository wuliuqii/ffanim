//! 渲染族命令：render（mp4 出片）/ timeline（调度表检查）/ frame（单帧 PNG）。

use crate::{demo, scenes};
use clap::Args;
use std::path::PathBuf;
use std::time::Instant;

/// 渲染场景为视频（mp4/gif/webm，画质预设对齐 manim -ql/-qm/-qh/-qk）
#[derive(Args)]
pub struct RenderArgs {
    /// 场景名（见 scenes.rs 注册表）
    #[arg(long, short)]
    pub scene: String,

    /// 输出路径（扩展名定格式：.mp4 / .gif / .webm）
    #[arg(long, short, default_value = "out/demo.mp4")]
    pub out: PathBuf,

    /// 帧率（--quality 给定时被预设 fps 覆盖）
    #[arg(long, default_value_t = 30)]
    pub fps: u32,

    /// 画质预设 ql/qm/qh/qk（manim 对应）：定输出分辨率 + fps + 各格式 CRF
    #[arg(long)]
    pub quality: Option<String>,

    /// 并行分段数（默认 = CPU 核数）
    #[arg(long)]
    pub segments: Option<usize>,

    /// 音轨文件（渲染后混入；片长延伸到音频长度，Duration::FromAudio 语义）
    #[arg(long)]
    pub audio: Option<PathBuf>,

    /// 片段导出：只渲染该时间段（秒，半开区间，如 "0..3.5"）
    #[arg(long)]
    pub section: Option<String>,

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

/// 渲染场景为视频（格式按输出扩展名路由，P1-9）。
pub fn render(args: RenderArgs) -> ffanim::Result<()> {
    let RenderArgs {
        scene,
        out,
        fps,
        quality,
        segments,
        audio,
        section,
        theme,
    } = args;
    // 画质预设：定输出分辨率/fps（manim 语义：质量档定 fps，时间轴按预设重建）
    let quality = quality
        .as_deref()
        .map(ffanim_encode::Quality::parse)
        .transpose()?;
    let (fps, quality) = match quality {
        Some(q) => {
            let s = q.spec();
            println!(
                "画质预设 {q:?}: 输出 {}×{} @{}fps（覆盖 --fps，时间轴按预设 fps 重建）",
                s.width, s.height, s.fps
            );
            (s.fps, Some(q))
        }
        None => (fps, None),
    };
    // GIF 不支持音轨（容器无音频流语义）
    if audio.is_some()
        && out
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("gif"))
    {
        return Err(ffanim::Error::Other(
            "GIF 输出不支持 --audio 音轨混入（容器限制），请改用 mp4/webm".into(),
        ));
    }
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
    let (w, h, fps) = (tl.width, tl.height, tl.fps);
    let tl = std::sync::Arc::new(tl);
    // --section：秒区间 → 帧区间（clamp 到全片），帧号平移走同一 encode 管道
    let (start_f, total) = match &section {
        Some(spec) => {
            if audio.is_some() {
                println!("注意：--section 与 --audio 同用时混入的是完整音轨，长度可能与片段不符");
            }
            let r = parse_section(spec, fps, tl.duration_frames)?;
            println!(
                "片段导出 {spec:?}: 帧 {}..{}（{} 帧 / {:.2}s）",
                r.start,
                r.end,
                r.end - r.start,
                (r.end - r.start) as f64 / f64::from(fps),
            );
            (r.start, r.end - r.start)
        }
        None => (0, tl.duration_frames),
    };
    let video = total as f64 / f64::from(fps);
    // 取帧闭包：渲染是纯函数，多线程调用安全（每帧独立 CPU 上下文）
    let frame_of = move |f: usize| -> Vec<u8> {
        let draw = ffanim_core::render_frame(&tl, start_f + f);
        ffanim_render::CpuBackend::new().render(&draw)
    };

    let opts = ffanim_encode::EncodeOptions {
        segments,
        quality,
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

/// 解析 `--section "起..止"`（秒，半开）为帧区间，clamp 到全片 [0, total]。
fn parse_section(spec: &str, fps: u32, total: usize) -> ffanim::Result<std::ops::Range<usize>> {
    let (a, b) = spec.split_once("..").ok_or_else(|| {
        ffanim::Error::Other(format!(
            "--section 需形如 \"起..止\"（秒），如 \"0..3.5\"，得 {spec:?}"
        ))
    })?;
    let bad = |v: &str| ffanim::Error::Other(format!("--section 数值非法: {v:?}（{spec:?}）"));
    let start: f64 = a.trim().parse().map_err(|_| bad(a))?;
    let end: f64 = b.trim().parse().map_err(|_| bad(b))?;
    if start < 0.0 || end < 0.0 {
        return Err(bad(spec));
    }
    let f = f64::from(fps);
    let s = ((start * f).round() as usize).min(total);
    let e = ((end * f).round() as usize).min(total);
    if s >= e {
        return Err(ffanim::Error::Other(format!(
            "--section 区间为空或越界: {spec:?}（全片 {total} 帧）"
        )));
    }
    Ok(s..e)
}

#[cfg(test)]
mod tests {
    use super::parse_section;

    #[test]
    fn section_秒转帧四舍五入() {
        assert_eq!(parse_section("0..3.5", 30, 105).unwrap(), 0..105);
        assert_eq!(parse_section("0.5..1.5", 30, 200).unwrap(), 15..45);
    }

    #[test]
    fn section_clamp到全片() {
        assert_eq!(parse_section("1..99", 30, 60).unwrap(), 30..60);
    }

    #[test]
    fn section_非法输入报错() {
        assert!(parse_section("3.5", 30, 100).is_err(), "缺 '..'");
        assert!(parse_section("a..b", 30, 100).is_err(), "非数字");
        assert!(parse_section("2..1", 30, 100).is_err(), "区间为空");
        assert!(parse_section("1..1", 30, 100).is_err(), "零长区间");
        assert!(parse_section("2..2", 30, 1).is_err(), "clamp 后为空");
    }
}
