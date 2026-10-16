//! 3D 曲面绘制示例：ThreeDSurfacePlot 验收（docs/07 §1.2，manim gallery 对照）。
//!
//! manim 原版：ThreeDAxes + 高斯面 `Surface(param_gauss, resolution=32)`，
//! 棋盘双色 + `fill_opacity=0.8`，静态机位展示。ffanim 对应：`axes3d` +
//! `parametric_surface`（σ=0.7 高斯峰，32×32 网格，棋盘双色派生自主题）。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example surface_plot`
//! （产出 out/surface_plot.mp4）

use ffanim::math::DVec3;
use ffanim::space3::{Axes3DOpts, Camera3D, SurfaceStyle, axes3d, parametric_surface, project};
use ffanim::{Anim, Item, Theme, Timeline};

const FPS: u32 = 30;
const W: f64 = 960.0;
const H: f64 = 540.0;

const SCENES: &[&str] = &["surface_plot"];

fn main() -> ffanim::Result<()> {
    let names: Vec<String> = std::env::args().skip(1).collect();
    let names: Vec<String> = if names.is_empty() {
        SCENES.iter().map(|s| (*s).to_string()).collect()
    } else {
        names
    };
    let theme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes/default.json");
    let theme = Theme::from_path(theme_path)?;
    let _ = std::fs::create_dir_all("out");
    for name in &names {
        if name != "surface_plot" {
            return Err(ffanim::Error::Other(format!("未知场景 {name}")));
        }
        let tl = surface_plot(&theme, FPS).compile();
        ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
        println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
    }
    Ok(())
}

/// ThreeDSurfacePlot 的 ffanim 版：高斯峰曲面棋盘 + 三轴，静态机位展示。
fn surface_plot(theme: &Theme, fps: u32) -> Timeline {
    let fg = theme.foreground();
    let accent = theme.accent();
    let bg = theme.background();
    let dim = fg.with_alpha(60);

    // 构建期一次生成图元表：轴系 + 高斯面（manim 同款 param_gauss，σ=0.7，峰高 2）
    let mut prims = axes3d(&Axes3DOpts {
        size: DVec3::new(3.5, 3.5, 2.5),
        grid: 3,
        axis: fg,
        axis_width: 2.5,
        grid_line: dim,
        grid_width: 1.0,
    });
    let sigma2 = 2.0 * 0.7 * 0.7;
    let gauss = |u: f64, v: f64| DVec3::new(u, v, 2.0 * (-(u * u + v * v) / sigma2).exp());
    // 棋盘双色 = 明暗替代；opacity 0.8 让背面隐约可见（manim 同款 fill_opacity）
    let dark = ffanim::Color::lerp(accent, bg, 0.55);
    prims.extend(parametric_surface(
        gauss,
        (-3.0, 3.0),
        (-3.0, 3.0),
        (32, 32),
        &SurfaceStyle::checkerboard(accent, dark, 0.8),
    ));

    let mut tl = Timeline::new(W as u32, H as u32, fps, bg);
    // 机位 manim 同款：phi=75°、theta=-30°；静态机位 = 构建期一次投影
    let cam = Camera3D {
        theta: -30f64.to_radians(),
        ..Camera3D::default()
    };
    let rig = tl.add(Item::group(project(&prims, &cam, W, H)));
    tl.play(Anim::fade_in(rig).during(0.8));
    tl.wait(1.0);
    tl
}
