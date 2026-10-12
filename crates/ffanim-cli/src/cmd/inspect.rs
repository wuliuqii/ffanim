//! 诊断与基准命令：inspect（采样统计）/ snapshot（基准图对比）。

use crate::scenes;
use clap::Args;
use ffanim_core::kurbo;
use ffanim_core::kurbo::Shape as _;
use std::path::PathBuf;

/// 诊断场景：字体加载 / 调度表 / 溢出画布采样（agent 排查用）
#[derive(Args)]
pub struct InspectArgs {
    /// 场景名
    #[arg(long, short)]
    pub scene: String,

    /// 采样帧数
    #[arg(long, default_value_t = 24)]
    pub samples: usize,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 渲染指定帧并与基准图比较（回归测试用）
#[derive(Args)]
pub struct SnapshotArgs {
    /// 场景名
    #[arg(long, short)]
    pub scene: String,

    /// 帧号
    #[arg(long, short, default_value_t = 0)]
    pub frame: usize,

    /// 保存/更新基准图（否则与现有基准比较）
    #[arg(long)]
    pub update: bool,

    /// 每通道允许的最大差值（抗锯齿/平台噪声容差）
    #[arg(long, default_value_t = 2)]
    pub tolerance: i32,

    /// 主题 JSON 路径
    #[arg(long, default_value = "themes/default.json")]
    pub theme: PathBuf,
}

/// 诊断场景：可见项与越界项统计。
pub fn inspect(args: InspectArgs) -> ffanim::Result<()> {
    let InspectArgs {
        scene,
        samples,
        theme,
    } = args;
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
        ffanim_render::walk_items(&draw.items, kurbo::Affine::IDENTITY, &mut |aff, item| {
            if let ffanim_core::Shape::Path(path) = &item.shape {
                visible += 1;
                let bbox = aff.transform_rect_bbox(path.bounding_box());
                let outside =
                    bbox.x1 < -1.0 || bbox.x0 > w + 1.0 || bbox.y1 < -1.0 || bbox.y0 > h + 1.0;
                if outside {
                    out_of_canvas += 1;
                }
            }
        });
        println!("  frame {frame:>4}: 可见 {visible} 项，越界 {out_of_canvas} 项");
    }
    Ok(())
}

/// 渲染指定帧并与基准图比较。
pub fn snapshot(args: SnapshotArgs) -> ffanim::Result<()> {
    let SnapshotArgs {
        scene,
        frame,
        update,
        tolerance,
        theme,
    } = args;
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
