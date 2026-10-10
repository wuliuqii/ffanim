//! ffanim CLI：M0 提供 `info`（主题加载验证）与 `--version`，后续里程碑逐步补全
//! render / frame / strip / onion / inspect / snapshot / timeline。

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
    }
}
