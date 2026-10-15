//! ffanim CLI：脚手架与 Agent 诊断工具链（纯薄壳）。
//!
//! 场景示范已迁至门面 crate 的 examples/（对齐 manim example_scenes 形态），
//! CLI 只保留与场景无关的通用子命令：new / audio / info / diff。
//! main.rs 仅保留 clap 定义与子命令分发；各命令实现在 `cmd/` 模块。

mod cmd;
mod scaffold;

use clap::{Parser, Subcommand};

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
    /// 生成脚手架项目（空目录 → 最小可跑工程）
    New(cmd::scaffold::NewArgs),

    /// 音频诊断：时长/电平/削波/静音段（agent 排查音轨）
    Audio(cmd::audio::AudioArgs),

    /// 加载并展示主题摘要（验证数据驱动管线）
    Info(cmd::scaffold::InfoArgs),

    /// 公式片段匹配预览（TransformMatchingTex 调度计划，agent/作者预检）
    Diff(cmd::formula::DiffArgs),
}

fn main() -> ffanim::Result<()> {
    // 日志埋点开关：默认静默（warn），调试时用 RUST_LOG 开启
    // 例：RUST_LOG=info 全部摘要；RUST_LOG=ffanim_core=trace 只看逐帧求值
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();
    match cli.command {
        Command::New(a) => cmd::scaffold::new(a),
        Command::Audio(a) => cmd::audio::audio(a),
        Command::Info(a) => cmd::scaffold::info(a),
        Command::Diff(a) => cmd::formula::diff(a),
    }
}
