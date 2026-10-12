//! Agent 视觉诊断命令：strip（接触表）/ onion（洋葱皮），转发 ffanim_render::tools。

use crate::{demo, scenes};
use clap::Args;
use std::path::PathBuf;

/// 接触表：等距采样帧拼成网格大图（agent 全局节奏/构图检查）
#[derive(Args)]
pub struct StripArgs {
    /// 场景名
    #[arg(long, short)]
    pub scene: String,

    /// 输出 PNG 路径
    #[arg(long, short, default_value = "")]
    pub out: String,

    /// 采样帧数
    #[arg(long, default_value_t = 12)]
    pub count: usize,

    /// 网格列数
    #[arg(long, default_value_t = 4)]
    pub cols: usize,

    /// 缩略图宽度（px，高度按画布比例）
    #[arg(long, default_value_t = 320)]
    pub thumb_width: u32,

    /// 以 JSON 输出报告
    #[arg(long)]
    pub json: bool,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 洋葱皮：基准帧叠加前后帧残影（运动轨迹检查）
#[derive(Args)]
pub struct OnionArgs {
    /// 场景名
    #[arg(long, short)]
    pub scene: String,

    /// 基准帧号（缺省取时间轴中点）
    #[arg(long, short)]
    pub frame: Option<usize>,

    /// 前后各取多少帧
    #[arg(long, default_value_t = 5)]
    pub spread: isize,

    /// 输出 PNG 路径
    #[arg(long, short, default_value = "")]
    pub out: String,

    /// 以 JSON 输出报告
    #[arg(long)]
    pub json: bool,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 接触表：等距采样帧拼成网格大图。
pub fn strip(args: StripArgs) -> ffanim::Result<()> {
    let StripArgs {
        scene,
        out,
        count,
        cols,
        thumb_width,
        json,
        theme,
    } = args;
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
    let report =
        ffanim_render::tools::strip(&tl, font.as_ref(), accent, out, count, cols, thumb_width)
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

/// 洋葱皮：基准帧叠加前后帧残影。
pub fn onion(args: OnionArgs) -> ffanim::Result<()> {
    let OnionArgs {
        scene,
        frame,
        spread,
        out,
        json,
        theme,
    } = args;
    let theme = ffanim::Theme::from_path(&theme)?;
    let tl = scenes::build(&scene, &theme, 30)?.compile();
    let last = tl.duration_frames.saturating_sub(1);
    let frame = frame.unwrap_or(last / 2); // 缺省取中点：动画进行中的典型时刻
    let out = if out.is_empty() {
        PathBuf::from(format!("frames/onion_{scene}_{frame:04}.png"))
    } else {
        PathBuf::from(out)
    };
    let report =
        ffanim_render::tools::onion(&tl, frame, spread, out).map_err(ffanim::Error::Other)?;
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
