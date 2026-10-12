//! 预览命令：winit 实时预览窗口（播放/暂停/逐帧/seek）。

use crate::scenes;
use clap::Args;

/// 实时预览窗口（播放/暂停/逐帧/seek）
#[derive(Args)]
pub struct PreviewArgs {
    /// 场景名
    #[arg(long, short)]
    pub scene: String,

    /// 帧率
    #[arg(long, default_value_t = 30)]
    pub fps: u32,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: std::path::PathBuf,
}

/// 启动实时预览窗口。
pub fn preview(args: PreviewArgs) -> ffanim::Result<()> {
    let PreviewArgs { scene, fps, theme } = args;
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
