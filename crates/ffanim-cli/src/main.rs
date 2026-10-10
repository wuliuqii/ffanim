//! ffanim CLI：M1 提供 `frame`（单帧 PNG 导出）与 `info`（主题加载验证），
//! 后续里程碑补全 render / strip / onion / inspect / snapshot / timeline。

mod demo;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
