//! 单摆演示：RK4 数值积分驱动物理动画（docs/07 §2.1 ODE 能力的验收场景）。
//!
//! 构建期 `rk4` 一次性积分 θ(t)（无阻尼单摆方程），渲染期 updater 纯查表
//! 重建摆组——两阶段模型的物理场景范式：积分不进渲染热路径。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example pendulum`（产出 out/pendulum_demo.mp4）

use ffanim::kurbo::{BezPath, Circle, PathEl, Point};
use ffanim::math::{Trajectory, rk4};
use ffanim::{Anim, Item, Theme, Timeline};

const FPS: u32 = 30;

const SCENES: &[&str] = &["pendulum_demo"];

fn main() -> ffanim::Result<()> {
    let names: Vec<String> = std::env::args().skip(1).map(Into::into).collect();
    let names: Vec<String> = if names.is_empty() {
        SCENES.iter().map(|s| (*s).to_string()).collect()
    } else {
        names
    };
    let theme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes/default.json");
    let theme = Theme::from_path(theme_path)?;
    let _ = std::fs::create_dir_all("out");
    for name in &names {
        if name != "pendulum_demo" {
            return Err(ffanim::Error::Other(format!("未知场景 {name}")));
        }
        let tl = pendulum_demo(&theme, FPS).compile();
        ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
        println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
    }
    Ok(())
}

/// 无阻尼单摆 θ'' = −(g/L)·sinθ，从 θ₀=0.7 rad 释放摆动 8 秒。
fn pendulum_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim::RateFunc;
    let (fg, accent, dim) = (
        theme.foreground(),
        theme.accent(),
        theme.foreground().with_alpha(90),
    );
    let sw = f64::from(theme.style.stroke_width);

    // 构建期积分：物理参数 g=9.8、L=1（θ 动力学只依赖 g/L），0..8s 共 1600 步
    let (g, l) = (9.8, 1.0);
    let traj: Trajectory = rk4(
        |_t, y| vec![y[1], -(g / l) * y[0].sin()],
        &[0.7, 0.0],
        0.0,
        8.0,
        1600,
    );

    let mut tl = Timeline::new(960, 540, fps, theme.background());
    let pivot = Point::new(480.0, 96.0);
    let len = 300.0; // 显示层摆长（px）；θ(t) 时间尺度由物理参数决定

    // 支点 + 竖直参考虚线（平衡位置）
    let anchor = tl.add(Item::circle(Circle::new(pivot, 5.0)).with_fill(fg));
    let mut guide = BezPath::new();
    guide.move_to(pivot);
    guide.line_to((pivot.x, pivot.y + len + 60.0));
    let guide_id = tl.add(
        Item::from_path(guide)
            .with_stroke(dim)
            .with_stroke_width(1.5),
    );
    tl.play(Anim::create(anchor).during(0.5));
    tl.play(Anim::fade_in(guide_id).during(0.5));

    // 摆组（杆 + 摆锤 + 角弧）：tracker 推进物理时间，updater 每帧查轨迹表重建
    let t = tl.add_tracker(0.0);
    let swing = tl.add_updater(&[t], move |v| {
        let theta = traj.at(v[0])[0];
        let bob = Point::new(pivot.x + len * theta.sin(), pivot.y + len * theta.cos());
        let mut rod = BezPath::new();
        rod.move_to(pivot);
        rod.line_to(bob);
        // 角弧：竖直方向到摆杆，半径 56px（θ 同号采样，直观显示摆角）
        let r = 56.0;
        let mut arc = BezPath::new();
        let n = 12;
        for i in 0..=n {
            let a = theta * (i as f64 / n as f64);
            let p = Point::new(pivot.x + r * a.sin(), pivot.y + r * a.cos());
            if i == 0 {
                arc.push(PathEl::MoveTo(p));
            } else {
                arc.push(PathEl::LineTo(p));
            }
        }
        Item::group(vec![
            Item::from_path(rod)
                .with_stroke(fg)
                .with_stroke_width(sw * 1.5),
            Item::circle(Circle::new(bob, 16.0)).with_fill(accent),
            Item::from_path(arc)
                .with_stroke(accent)
                .with_stroke_width(sw),
        ])
    });
    tl.play(Anim::fade_in(swing).during(0.6));

    // 摆动 8 秒（≈2.6 个周期），末尾淡出
    tl.play(Anim::set_value(t, 8.0).during(8.0).rate(RateFunc::Linear));
    tl.play(Anim::fade_out(swing).during(0.5));
    tl.wait(0.3);
    tl
}
