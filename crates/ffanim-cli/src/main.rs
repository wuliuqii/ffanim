//! ffanim CLI：M2 提供 `render`（mp4 渲染）/ `timeline`（时间轴检查）/ `frame`（单帧 PNG）
//! 与 `info`（主题加载验证），后续里程碑补全 strip / onion / inspect / snapshot。

mod demo;
mod scenes;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Instant;

#[derive(Parser)]
#[command(
    name = "ffanim",
    version = ffanim::VERSION,
    about = "用 Rust 写数学动画"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 渲染场景为 mp4 视频
    Render {
        /// 场景名（见 scenes.rs 注册表）
        #[arg(long, short)]
        scene: String,

        /// 输出 mp4 路径
        #[arg(long, short, default_value = "out/demo.mp4")]
        out: PathBuf,

        /// 帧率
        #[arg(long, default_value_t = 30)]
        fps: u32,

        /// 并行分段数（默认 = CPU 核数）
        #[arg(long)]
        segments: Option<usize>,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 打印编译后时间轴（动画调度表，供 agent 检查）
    Timeline {
        /// 场景名
        #[arg(long, short)]
        scene: String,

        /// 以 JSON 输出
        #[arg(long)]
        json: bool,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 渲染单帧示例场景为 PNG（M1 验收场景：三角形 + 文字 + 圆）
    Frame {
        /// 输出 PNG 路径
        #[arg(long, short, default_value = "frames/demo.png")]
        out: PathBuf,

        /// 画布宽度（px）
        #[arg(long, default_value_t = 960)]
        width: u32,

        /// 画布高度（px）
        #[arg(long, default_value_t = 540)]
        height: u32,

        /// 渲染后端（cpu / gpu，gpu 需编译时启用 gpu 特性）
        #[arg(long, default_value = "cpu")]
        backend: String,

        /// 字体文件路径（缺省时搜索系统常见位置）
        #[arg(long, short)]
        font: Option<PathBuf>,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 加载并展示主题摘要（验证数据驱动管线）
    Info {
        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },
}

fn main() -> ffanim::Result<()> {
    // 日志埋点开关：默认静默（warn），调试时用 RUST_LOG 开启
    // 例：RUST_LOG=info 全部摘要；RUST_LOG=ffanim_core=trace 只看逐帧求值
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();
    match cli.command {
        Command::Info { theme } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let bg = theme.background()?;
            println!("主题: {}", theme.name);
            println!("背景色: #{:02x}{:02x}{:02x}{:02x}", bg.r, bg.g, bg.b, bg.a);
            println!("描边宽度: {}", theme.style.stroke_width);
            println!(
                "公式字号: {}pt / 字体: {}",
                theme.formula.font_size_pt, theme.formula.font_family
            );
            Ok(())
        }

        Command::Render {
            scene,
            out,
            fps,
            segments,
            theme,
        } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let tl = scenes::build(&scene, &theme, fps)?.compile();
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
            ffanim_encode::encode_video(w, h, fps, total, &out, &opts, &frame_of)?;
            let elapsed = start.elapsed().as_secs_f64();
            println!(
                "已输出 {out:?}（渲染 {elapsed:.2}s / 视频时长 {video:.2}s，实时率 {:.1}x）",
                video / elapsed
            );
            Ok(())
        }

        Command::Timeline { scene, json, theme } => {
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

        Command::Frame {
            out,
            width,
            height,
            backend,
            font,
            theme,
        } => {
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
    }
}

// M1 阶段：CLI 暂只链接 CPU 后端；GPU 后端经 feature 暴露后接入
