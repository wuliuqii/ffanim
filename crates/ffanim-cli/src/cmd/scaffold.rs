//! 脚手架与元信息命令：new（生成工程）/ info（主题摘要）。

use crate::scaffold;
use clap::Args;
use std::path::PathBuf;

/// 生成脚手架项目（空目录 → 最小可跑工程）
#[derive(Args)]
pub struct NewArgs {
    /// 项目目录（最后一段作为项目名）
    pub dir: PathBuf,

    /// 场景模板
    #[arg(long, default_value = "minimal")]
    pub template: String,

    /// ffanim 依赖指向本仓库源码（本机验证用，免 crates.io 发布）
    #[arg(long)]
    pub dev: bool,
}

/// 加载并展示主题摘要（验证数据驱动管线）
#[derive(Args)]
pub struct InfoArgs {
    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 生成脚手架项目。
pub fn new(args: NewArgs) -> ffanim::Result<()> {
    let NewArgs { dir, template, dev } = args;
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

/// 加载并展示主题摘要。
pub fn info(args: InfoArgs) -> ffanim::Result<()> {
    let InfoArgs { theme } = args;
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
