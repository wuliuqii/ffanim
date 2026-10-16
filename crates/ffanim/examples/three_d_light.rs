//! 3D 光源位置示例：ThreeDLightSourcePosition 验收（docs/07 §1.2，manim gallery 对照）。
//!
//! manim 原版：ThreeDAxes + 球面 `Surface`（棋盘双色），移动 `camera.light_source`
//! 演示明暗。ffanim 对应：`axes3d` 构造件 + `parametric_surface` 球面棋盘——
//! 投影模拟无光照（非目标），明暗感由棋盘双色承担（亮面 accent / 暗面 accent
//! 向背景混合，全从主题派生）；`light_source.move_to` 一行不追。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example three_d_light`
//! （产出 out/three_d_light.mp4）

use std::f64::consts::{PI, TAU};

use ffanim::math::DVec3;
use ffanim::space3::{Axes3DOpts, Camera3D, SurfaceStyle, axes3d, parametric_surface, project};
use ffanim::{Anim, Item, Theme, Timeline};

const FPS: u32 = 30;
const W: f64 = 960.0;
const H: f64 = 540.0;

const SCENES: &[&str] = &["three_d_light"];

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
        if name != "three_d_light" {
            return Err(ffanim::Error::Other(format!("未知场景 {name}")));
        }
        let tl = three_d_light(&theme, FPS).compile();
        ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
        println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
    }
    Ok(())
}

/// ThreeDLightSourcePosition 的 ffanim 版：球面棋盘 + 三轴，静态机位展示。
fn three_d_light(theme: &Theme, fps: u32) -> Timeline {
    let fg = theme.foreground();
    let accent = theme.accent();
    let bg = theme.background();
    let dim = fg.with_alpha(60);

    // 构建期一次生成图元表：轴系 + 球面棋盘（半径 1.5 上半球，manim 同款参数）
    let mut prims = axes3d(&Axes3DOpts {
        size: DVec3::new(3.0, 3.0, 2.5),
        grid: 2,
        axis: fg,
        axis_width: 2.5,
        grid_line: dim,
        grid_width: 1.0,
    });
    // 棋盘双色 = 明暗替代：亮面 accent，暗面向背景混 55%（主题派生，零硬编码）
    let dark = ffanim::Color::lerp(accent, bg, 0.55);
    let sphere = |u: f64, v: f64| 1.5 * DVec3::new(u.cos() * v.cos(), u.sin() * v.cos(), v.sin());
    prims.extend(parametric_surface(
        sphere,
        (0.0, TAU),
        (0.0, PI),
        (16, 16),
        &SurfaceStyle::checkerboard(accent, dark, 1.0),
    ));

    let mut tl = Timeline::new(W as u32, H as u32, fps, bg);
    // 机位 manim 同款：phi=75°、theta=30°，即 Camera3D::default()。
    // 静态机位 = 构建期一次投影（产物即普通 Item，无需 updater 每帧重投影）
    let rig = tl.add(Item::group(project(&prims, &Camera3D::default(), W, H)));
    tl.play(Anim::fade_in(rig).during(0.8));
    tl.wait(1.0);
    tl
}
