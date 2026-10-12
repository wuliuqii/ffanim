//! ffanim CLI：渲染与 Agent 验证工具链。
//!
//! main.rs 仅保留 clap 定义与子命令分发；各命令实现在 `cmd/` 模块
//! （docs/04-项目优化规划.md §4.2）。

mod cmd;
mod demo;
mod scaffold;
mod scenes;

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
    /// 渲染场景为 mp4 视频
    Render(cmd::render::RenderArgs),

    /// 打印编译后时间轴（动画调度表，供 agent 检查）
    Timeline(cmd::render::TimelineArgs),

    /// 诊断场景：字体加载 / 调度表 / 溢出画布采样（agent 排查用）
    Inspect(cmd::inspect::InspectArgs),

    /// 渲染指定帧并与基准图比较（回归测试用）
    Snapshot(cmd::inspect::SnapshotArgs),

    /// 实时预览窗口（播放/暂停/逐帧/seek）
    Preview(cmd::preview::PreviewArgs),

    /// 渲染单帧示例场景为 PNG（M1 验收场景：三角形 + 文字 + 圆）
    Frame(cmd::render::FrameArgs),

    /// 接触表：等距采样帧拼成网格大图（agent 全局节奏/构图检查）
    Strip(cmd::tools::StripArgs),

    /// 洋葱皮：基准帧叠加前后帧残影（运动轨迹检查）
    Onion(cmd::tools::OnionArgs),

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
        Command::Render(a) => cmd::render::render(a),
        Command::Timeline(a) => cmd::render::timeline(a),
        Command::Frame(a) => cmd::render::frame(a),
        Command::Inspect(a) => cmd::inspect::inspect(a),
        Command::Snapshot(a) => cmd::inspect::snapshot(a),
        Command::Preview(a) => cmd::preview::preview(a),
        Command::Strip(a) => cmd::tools::strip(a),
        Command::Onion(a) => cmd::tools::onion(a),
        Command::New(a) => cmd::scaffold::new(a),
        Command::Audio(a) => cmd::audio::audio(a),
        Command::Info(a) => cmd::scaffold::info(a),
        Command::Diff(a) => cmd::formula::diff(a),
    }
}

// M1 阶段：CLI 暂只链接 CPU 后端；GPU 后端经 feature 暴露后接入
