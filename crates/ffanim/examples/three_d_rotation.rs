//! 3D 相机环境自转：ThreeDCameraRotation 验收（docs/07 §1.2，manim gallery 对照）。
//!
//! manim 原版：ThreeDAxes 加 Circle，配合 `begin_ambient_camera_rotation(rate=0.1)`
//! 与 `move_camera`。ffanim 对应：三轴、z=0 网格与单位圆用 `polyline3` 拼图元表，
//! 相机方位角 theta 挂 tracker——ambient rotation 即匀速推进，move_camera 即
//! Smooth 回位；渲染期 updater 纯查表重投影（`project`）与画家算法排序。
//! 相机动画零新机制，全部由现有 tracker 与 `Anim::set_value` 承担。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example three_d_rotation`
//! （产出 out/three_d_rotation.mp4）

use std::f64::consts::TAU;

use ffanim::math::DVec3;
use ffanim::space3::{Camera3D, Prim3D, PrimStyle, polyline3, project};
use ffanim::{Anim, Color, Item, RateFunc, Theme, Timeline};

const FPS: u32 = 30;
const W: f64 = 960.0;
const H: f64 = 540.0;

const SCENES: &[&str] = &["three_d_rotation"];

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
        if name != "three_d_rotation" {
            return Err(ffanim::Error::Other(format!("未知场景 {name}")));
        }
        let tl = three_d_rotation(&theme, FPS).compile();
        ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
        println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
    }
    Ok(())
}

/// 构建期一次生成 3D 图元表：三轴 + z=0 方格网格 + 单位圆（旋转深度感全靠网格）。
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

/// ThreeDCameraRotation 的 ffanim 版：环境自转 0.1 圈/s × 3s，再 Smooth 回位。
fn three_d_rotation(theme: &Theme, fps: u32) -> Timeline {
    let fg = theme.foreground();
    let accent = theme.accent();
    let dim = fg.with_alpha(60);
    let prims = scene_prims(fg, accent, dim);

    let mut tl = Timeline::new(W as u32, H as u32, fps, theme.background());

    // 相机机位：manim 同款 phi=75°、theta=30° 起步；方位角挂 tracker 可动画
    let start = 30f64.to_radians();
    let theta = tl.add_tracker(start);
    let rig = tl.add_updater(&[theta], move |v| {
        let cam = Camera3D {
            theta: v[0],
            ..Camera3D::default()
        };
        Item::group(project(&prims, &cam, W, H))
    });
    tl.play(Anim::fade_in(rig).during(0.5));

    // ambient rotation：rate=0.1 圈/s × 3s（manim 同款参数，Linear 匀速）
    tl.play(
        Anim::set_value(theta, start + 0.1 * TAU * 3.0)
            .during(3.0)
            .rate(RateFunc::Linear),
    );
    // move_camera：Smooth 回到起始方位角
    tl.play(
        Anim::set_value(theta, start)
            .during(1.0)
            .rate(RateFunc::Smooth),
    );
    tl.wait(0.5);
    tl
}
