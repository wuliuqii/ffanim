//! 固定屏上层：FixedInFrameMObjectTest 验收（docs/07 §1.2，manim gallery 对照收官例）。
//!
//! manim 原版：3D 层（ThreeDAxes + Circle）轴绕 z 转 90°、圆平移到 (1,1,0)，
//! 左上角 `DecimalNumber` 通过 `add_fixed_in_frame_mobjects` 固定屏上随时间滚动。
//! ffanim 对应（**零新 API**）：3D 层 updater 每帧重投影；2D 层数字是不进
//! `project` 的普通 updater Item——不参与投影即天然固定屏上，无需任何固定层机制。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example fixed_in_frame`
//! （产出 out/fixed_in_frame.mp4）

use std::f64::consts::{FRAC_PI_2, TAU};

use ffanim::kurbo::Point;
use ffanim::math::DVec3;
use ffanim::space3::{
    Axes3DOpts, Camera3D, Prim3D, PrimStyle, axes3d, polygon3, polyline3, project,
};
use ffanim::typst::{DecimalNumberOptions, TypstEngine};
use ffanim::{Anim, Item, RateFunc, Theme, Timeline};

const FPS: u32 = 30;
const W: f64 = 960.0;
const H: f64 = 540.0;

const SCENES: &[&str] = &["fixed_in_frame"];

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
        if name != "fixed_in_frame" {
            return Err(ffanim::Error::Other(format!("未知场景 {name}")));
        }
        let tl = fixed_in_frame(&theme, FPS)?;
        let tl = tl.compile();
        ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
        println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
    }
    Ok(())
}

/// FixedInFrameMObjectTest 的 ffanim 版：3D 层动、2D 层固定，对照收官。
fn fixed_in_frame(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    let fg = theme.foreground();
    let accent = theme.accent();
    let bg = theme.background();
    let dim = fg.with_alpha(60);

    // 3D 层基座：轴系构建期一次生成（旋转基座），相机 manim 同款 phi=75°/theta=-45°
    let base_axes = axes3d(&Axes3DOpts {
        size: DVec3::new(3.0, 3.0, 2.5),
        grid: 2,
        axis: fg,
        axis_width: 2.5,
        grid_line: dim,
        grid_width: 1.0,
    });
    let cam = Camera3D {
        theta: -45f64.to_radians(),
        ..Camera3D::default()
    };

    let mut tl = Timeline::new(W as u32, H as u32, fps, bg);

    // 3D 层：进度 tracker p 派生同窗动画——轴绕 z 转 90°（原版 Rotate(axes, π/2,
    // Z_AXIS)）+ 单位圆平移到 (1,1,0)（原版 circle.animate.move_to([1,1,0])）
    let p = tl.add_tracker(0.0);
    let rig = tl.add_updater(&[p], move |v| {
        let t = v[0];
        let (s, c) = (t * FRAC_PI_2).sin_cos();
        let rz = |q: DVec3| DVec3::new(q.x * c - q.y * s, q.x * s + q.y * c, q.z);
        let mut prims: Vec<Prim3D> = base_axes
            .iter()
            .map(|prim| match prim {
                Prim3D::Polyline { pts, style } => {
                    polyline3(pts.iter().map(|q| rz(*q)), style.clone())
                }
                Prim3D::Polygon { pts, style } => {
                    polygon3(pts.iter().map(|q| rz(*q)), style.clone())
                }
            })
            .collect();
        let circle = (0..=64).map(|i| {
            let a = i as f64 / 64.0 * TAU;
            rz(DVec3::new(a.cos(), a.sin(), 0.0)) + DVec3::new(t, t, 0.0)
        });
        prims.push(polyline3(circle, PrimStyle::line(accent, 3.0)));
        Item::group(project(&prims, &cam, W, H))
    });
    tl.play(Anim::fade_in(rig).during(0.5));

    // 2D 固定层（零 API）：不进投影的普通 updater Item 天然固定屏上；左上角
    // 数字随进度滚动（原版 DecimalNumber + set_value(self.time) 对应，此处
    // 显示轴旋转角 0°→90° 与 3D 层联动）
    let sig = TypstEngine::decimal_signal(
        Point::new(90.0, 70.0),
        DecimalNumberOptions {
            decimals: 0,
            group_with_commas: false,
            include_sign: false,
            font_size_pt: 40.0,
            text_color: fg,
        },
    )?;
    let _num = tl.add_updater(&[p], move |v| sig(&[v[0] * 90.0]));

    // 原版同窗 play（Smooth）：轴转 + 圆移 2s
    tl.play(Anim::set_value(p, 1.0).during(2.0).rate(RateFunc::Smooth));
    tl.wait(1.0);
    Ok(tl)
}
