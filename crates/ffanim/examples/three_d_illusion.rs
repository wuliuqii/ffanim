//! 3D 相机错觉振荡：ThreeDCameraIllusionRotation 验收（docs/07 §1.2，manim gallery 对照）。
//!
//! manim 原版：`begin_3dillusion_camera_rotation(rate)`——内部 value tracker 匀速推进
//! 相位，theta 围绕起始角做 `theta0 + sin(phase)` 正弦摆动（幅度固定 1 rad，rate 只控频率）。
//! ffanim 对应：相位挂 tracker + `Anim::set_value` 匀速推进，updater 纯函数求值
//! `theta = theta0 + sin(phase)`——零新动画机制，振荡即查表。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example three_d_illusion`
//! （产出 out/three_d_illusion.mp4）

use ffanim::math::DVec3;
use ffanim::space3::{Camera3D, Prim3D, PrimStyle, polyline3, project};
use ffanim::{Anim, Color, Item, RateFunc, Theme, Timeline};

use std::f64::consts::TAU;

const FPS: u32 = 30;
const W: f64 = 960.0;
const H: f64 = 540.0;

const SCENES: &[&str] = &["three_d_illusion"];

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
        if name != "three_d_illusion" {
            return Err(ffanim::Error::Other(format!("未知场景 {name}")));
        }
        let tl = three_d_illusion(&theme, FPS).compile();
        ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
        println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
    }
    Ok(())
}

/// 构建期一次生成 3D 图元表：三轴 + z=0 方格网格 + 单位圆（深度感靠网格，
/// 与 three_d_rotation 同款场景，manim 原版为 ThreeDAxes + Circle）。
fn scene_prims(fg: Color, accent: Color, dim: Color) -> Vec<Prim3D> {
    let axis = |end: DVec3| polyline3([DVec3::ZERO, end], PrimStyle::line(fg, 2.5));
    let mut prims = vec![
        axis(DVec3::new(3.0, 0.0, 0.0)),
        axis(DVec3::new(0.0, 3.0, 0.0)),
        axis(DVec3::new(0.0, 0.0, 2.5)),
    ];
    // z=0 方格网格（-2..2，步长 1）
    for i in -2..=2i32 {
        let c = i as f64;
        prims.push(polyline3(
            [DVec3::new(c, -2.0, 0.0), DVec3::new(c, 2.0, 0.0)],
            PrimStyle::line(dim, 1.0),
        ));
        prims.push(polyline3(
            [DVec3::new(-2.0, c, 0.0), DVec3::new(2.0, c, 0.0)],
            PrimStyle::line(dim, 1.0),
        ));
    }
    // z=0 平面单位圆（64 段折线闭合）
    let circle = (0..=64).map(|i| {
        let a = i as f64 / 64.0 * TAU;
        DVec3::new(a.cos(), a.sin(), 0.0)
    });
    prims.push(polyline3(circle, PrimStyle::line(accent, 3.0)));
    prims
}

/// ThreeDCameraIllusionRotation 的 ffanim 版：theta 围绕起始角 ±1 rad 正弦摆动 4s。
fn three_d_illusion(theme: &Theme, fps: u32) -> Timeline {
    let fg = theme.foreground();
    let accent = theme.accent();
    let dim = fg.with_alpha(60);
    let prims = scene_prims(fg, accent, dim);

    let mut tl = Timeline::new(W as u32, H as u32, fps, theme.background());

    // manim 同款机位：phi=75°、theta0=-45°；相位 tracker 匀速推进，theta=theta0+sin(phase)
    let theta0 = -45f64.to_radians();
    let phase = tl.add_tracker(0.0);
    let rig = tl.add_updater(&[phase], move |v| {
        let cam = Camera3D {
            theta: theta0 + v[0].sin(),
            ..Camera3D::default()
        };
        Item::group(project(&prims, &cam, W, H))
    });
    tl.play(Anim::fade_in(rig).during(0.5));

    // illusion：rate=2 rad/s × 4s（manim 同款参数），相位匀速推进
    tl.play(
        Anim::set_value(phase, 2.0 * 4.0)
            .during(4.0)
            .rate(RateFunc::Linear),
    );
    tl.wait(0.5);
    tl
}
