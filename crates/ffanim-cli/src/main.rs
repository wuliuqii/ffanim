//! ffanim CLI：M2 提供 `render`（mp4 渲染）/ `timeline`（时间轴检查）/ `frame`（单帧 PNG）
//! 与 `info`（主题加载验证），后续里程碑补全 strip / onion / inspect / snapshot。

mod demo;
mod scaffold;
mod scenes;
mod tools;

use clap::{Parser, Subcommand};
use ffanim_core::kurbo;
use ffanim_core::kurbo::Shape as _;
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

        /// 音轨文件（渲染后混入；片长延伸到音频长度，Duration::FromAudio 语义）
        #[arg(long)]
        audio: Option<PathBuf>,

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

    /// 诊断场景：字体加载 / 调度表 / 溢出画布采样（agent 排查用）
    Inspect {
        /// 场景名
        #[arg(long, short)]
        scene: String,

        /// 采样帧数
        #[arg(long, default_value_t = 24)]
        samples: usize,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 渲染指定帧并与基准图比较（回归测试用）
    Snapshot {
        /// 场景名
        #[arg(long, short)]
        scene: String,

        /// 帧号
        #[arg(long, short, default_value_t = 0)]
        frame: usize,

        /// 保存/更新基准图（否则与现有基准比较）
        #[arg(long)]
        update: bool,

        /// 每通道允许的最大差值（抗锯齿/平台噪声容差）
        #[arg(long, default_value_t = 2)]
        tolerance: i32,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 实时预览窗口（播放/暂停/逐帧/seek）
    Preview {
        /// 场景名
        #[arg(long, short)]
        scene: String,

        /// 帧率
        #[arg(long, default_value_t = 30)]
        fps: u32,

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

    /// 接触表：等距采样帧拼成网格大图（agent 全局节奏/构图检查）
    Strip {
        /// 场景名
        #[arg(long, short)]
        scene: String,

        /// 输出 PNG 路径
        #[arg(long, short, default_value = "")]
        out: String,

        /// 采样帧数
        #[arg(long, default_value_t = 12)]
        count: usize,

        /// 网格列数
        #[arg(long, default_value_t = 4)]
        cols: usize,

        /// 缩略图宽度（px，高度按画布比例）
        #[arg(long, default_value_t = 320)]
        thumb_width: u32,

        /// 以 JSON 输出报告
        #[arg(long)]
        json: bool,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 洋葱皮：基准帧叠加前后帧残影（运动轨迹检查）
    Onion {
        /// 场景名
        #[arg(long, short)]
        scene: String,

        /// 基准帧号（缺省取时间轴中点）
        #[arg(long, short)]
        frame: Option<usize>,

        /// 前后各取多少帧
        #[arg(long, default_value_t = 5)]
        spread: isize,

        /// 输出 PNG 路径
        #[arg(long, short, default_value = "")]
        out: String,

        /// 以 JSON 输出报告
        #[arg(long)]
        json: bool,

        /// 主题 JSON 路径
        #[arg(long, default_value = "themes/default.json")]
        theme: PathBuf,
    },

    /// 生成脚手架项目（空目录 → 最小可跑工程）
    New {
        /// 项目目录（最后一段作为项目名）
        dir: PathBuf,

        /// 场景模板
        #[arg(long, default_value = "minimal")]
        template: String,

        /// ffanim 依赖指向本仓库源码（本机验证用，免 crates.io 发布）
        #[arg(long)]
        dev: bool,
    },

    /// 音频诊断：时长/电平/削波/静音段（agent 排查音轨）
    Audio {
        /// 音频文件路径
        #[arg(long, short)]
        input: PathBuf,

        /// 以 JSON 输出报告
        #[arg(long)]
        json: bool,
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
        Command::New { dir, template, dev } => {
            let files = scaffold::generate(&dir, &template, dev)?;
            println!("已生成项目 {}（模板 {template}）:", dir.display());
            for f in &files {
                println!("  {}", f.display());
            }
            println!(
                "\n下一步: cd {} && cargo run（产物 out/demo.mp4）",
                dir.display()
            );
            Ok(())
        }

        Command::Audio { input, json } => {
            let report = ffanim_audio::analyze(&input)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "input": input,
                        "duration": report.duration,
                        "sample_rate": report.sample_rate,
                        "channels": report.channels,
                        "rms_db": report.rms_db,
                        "peak_db": report.peak_db,
                        "clipped_samples": report.clipped_samples,
                        "silence": report.silence,
                    })
                );
            } else {
                println!(
                    "音频 {input:?}: {:.2}s @{}Hz（{} 声道）",
                    report.duration, report.sample_rate, report.channels
                );
                println!(
                    "电平: RMS {:.1}dBFS / 峰值 {:.1}dBFS，削波 {} 样本",
                    report.rms_db, report.peak_db, report.clipped_samples
                );
                if report.silence.is_empty() {
                    println!("静音段: 无");
                } else {
                    println!("静音段:");
                    for (s, e) in &report.silence {
                        println!("  {s:.2}s .. {e:.2}s");
                    }
                }
            }
            Ok(())
        }

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
            audio,
            theme,
        } => {
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

        Command::Inspect {
            scene,
            samples,
            theme,
        } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let tl = scenes::build(&scene, &theme, 30)?.compile();
            let (w, h) = (f64::from(tl.width), f64::from(tl.height));
            println!(
                "场景 {scene}: {} 帧 / {:.2}s / {} 个动画 / {} 个顶层项",
                tl.duration_frames,
                tl.duration_frames as f64 / f64::from(tl.fps),
                tl.anims.len(),
                tl.items.len()
            );

            // 等距采样：逐帧统计可见项数与完全越出画布的项数（bbox 容差 1px）
            let samples = samples.max(2);
            for k in 0..samples {
                let frame = k * tl.duration_frames.saturating_sub(1) / (samples - 1);
                let draw = ffanim_core::render_frame(&tl, frame);
                let mut visible = 0usize;
                let mut out_of_canvas = 0usize;
                ffanim_render::walk_items(
                    &draw.items,
                    kurbo::Affine::IDENTITY,
                    &mut |aff, item| {
                        if let ffanim_core::Shape::Path(path) = &item.shape {
                            visible += 1;
                            let bbox = aff.transform_rect_bbox(path.bounding_box());
                            let outside = bbox.x1 < -1.0
                                || bbox.x0 > w + 1.0
                                || bbox.y1 < -1.0
                                || bbox.y0 > h + 1.0;
                            if outside {
                                out_of_canvas += 1;
                            }
                        }
                    },
                );
                println!("  frame {frame:>4}: 可见 {visible} 项，越界 {out_of_canvas} 项");
            }
            Ok(())
        }

        Command::Snapshot {
            scene,
            frame,
            update,
            tolerance,
            theme,
        } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let tl = scenes::build(&scene, &theme, 30)?.compile();
            let frame = frame.min(tl.duration_frames.saturating_sub(1));
            let draw = ffanim_core::render_frame(&tl, frame);
            use ffanim_render::RenderBackend;
            let rgba = ffanim_render::CpuBackend::new().render(&draw);
            let (w, h) = (tl.width, tl.height);
            let baseline = PathBuf::from(format!("frames/baseline/{scene}_{frame:04}.png"));

            if update || !baseline.exists() {
                ffanim_render::save_png(&baseline, w, h, &rgba)
                    .map_err(|e| ffanim::Error::ThemeIo(e, baseline.clone()))?;
                println!("基准图已保存: {baseline:?}（{w}x{h}，frame {frame}）");
                return Ok(());
            }

            // 与基准逐像素比较：RGB 任一通道差 > tolerance 记为差异像素
            let (bw, bh, base) = ffanim_render::load_png(&baseline)
                .map_err(|e| ffanim::Error::ThemeIo(e, baseline.clone()))?;
            if (bw, bh) != (w, h) {
                return Err(ffanim::Error::Other(format!(
                    "基准图尺寸 {bw}x{bh} 与渲染 {w}x{h} 不一致"
                )));
            }
            let diff_pixels = base
                .iter()
                .zip(&rgba)
                .enumerate()
                .filter(|(i, (a, b))| {
                    // 跳过 alpha 通道（i % 4 == 3），只比 RGB
                    i % 4 != 3 && i32::from(u8::abs_diff(**a, **b)) > tolerance
                })
                .count();
            let total = w as usize * h as usize;
            let ratio = diff_pixels as f64 / total as f64 * 100.0;
            if diff_pixels == 0 {
                println!("PASS: 与基准 {baseline:?} 一致");
            } else {
                println!(
                    "FAIL: {diff_pixels}/{total} 像素差异（{ratio:.3}%，容差 {tolerance}）— 基准 {baseline:?}"
                );
            }
            Ok(())
        }

        Command::Preview { scene, fps, theme } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let tl = scenes::build(&scene, &theme, fps)?.compile();
            let accent = theme.accent()?;
            println!(
                "预览 {scene}: {} 帧 / {:.2}s（空格播放，←/→ 逐帧，Q 退出）",
                tl.duration_frames,
                tl.duration_frames as f64 / f64::from(tl.fps)
            );
            ffanim_preview::run(tl, accent).map_err(ffanim::Error::Other)?;
            Ok(())
        }

        Command::Strip {
            scene,
            out,
            count,
            cols,
            thumb_width,
            json,
            theme,
        } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let tl = scenes::build(&scene, &theme, 30)?.compile();
            // 字体可选：加载失败只丢帧号标注，不阻塞诊断图
            let font = demo::load_font(None).ok();
            let out = if out.is_empty() {
                PathBuf::from(format!("frames/strip_{scene}.png"))
            } else {
                PathBuf::from(out)
            };
            let accent = theme.accent()?;
            let report = tools::strip(&tl, font.as_ref(), accent, out, count, cols, thumb_width)
                .map_err(ffanim::Error::Other)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "scene": scene,
                        "out": report.out,
                        "frames": report.frames,
                        "cols": report.cols,
                        "thumb": { "width": report.thumb.0, "height": report.thumb.1 },
                    })
                );
            } else {
                println!(
                    "已输出 {0:?}（{1} 帧采样 × {2} 列，缩略 {3}x{4}）",
                    report.out,
                    report.frames.len(),
                    report.cols,
                    report.thumb.0,
                    report.thumb.1
                );
                println!("采样帧: {:?}", report.frames);
            }
            Ok(())
        }

        Command::Onion {
            scene,
            frame,
            spread,
            out,
            json,
            theme,
        } => {
            let theme = ffanim::Theme::from_path(&theme)?;
            let tl = scenes::build(&scene, &theme, 30)?.compile();
            let last = tl.duration_frames.saturating_sub(1);
            let frame = frame.unwrap_or(last / 2); // 缺省取中点：动画进行中的典型时刻
            let out = if out.is_empty() {
                PathBuf::from(format!("frames/onion_{scene}_{frame:04}.png"))
            } else {
                PathBuf::from(out)
            };
            let report = tools::onion(&tl, frame, spread, out).map_err(ffanim::Error::Other)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "scene": scene,
                        "out": report.out,
                        "frame": report.frame,
                        "spread": report.spread,
                    })
                );
            } else {
                println!(
                    "已输出 {0:?}（基准帧 {1}，残影 ±{2} 帧）",
                    report.out, report.frame, report.spread
                );
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
