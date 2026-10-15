//! 坐标系场景：网格淡入 → 轴生长 → 正弦图像随 ValueTracker 振幅呼吸。
//!
//! 运行：`cargo run`（需要 PATH 里有 ffmpeg），产物 out/demo.mp4。

use ffanim::{Anim, Axes, Color, Item, Timeline, Theme};
use ffanim::kurbo::Rect;

fn main() -> ffanim::Result<()> {
    // 数据驱动：颜色与描边宽度全部来自主题 JSON，代码零硬编码颜色
    let theme = Theme::from_path("themes/default.json")?;
    let accent = theme.accent();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, 30, theme.background());

    // 坐标系：x ∈ [-3,3]，y ∈ [-1.2,1.2]，居中放置
    let axes = Axes::new((-3.0, 3.0), (-1.2, 1.2))
        .with_size(840.0, 400.0)
        .with_center(f64::from(w) / 2.0, f64::from(h) / 2.0);

    // 网格：淡入
    let grid = tl.add(axes.grid_item(theme.foreground().with_alpha(40), 1.0));
    tl.play(Anim::fade_in(grid).during(0.6));

    // 轴与刻度：错开 0.4s 后生长
    let ax = tl.add(axes.to_item(&theme.style));
    tl.overlap(0.4);
    tl.play(Anim::create(ax).during(1.0));

    // 函数图像：挂 updater 到振幅 tracker，渲染期逐帧按值重建
    let amp = tl.add_tracker(1.0);
    let graph = tl.add_updater(&[amp], move |v| {
        axes.plot(|x| v[0] * (x * std::f64::consts::FRAC_PI_2).sin(), (-3.0, 3.0))
            .with_stroke(accent)
            .with_stroke_width(3.0)
    });
    tl.play(Anim::create(graph).during(1.2));
    tl.play(Anim::set_value(amp, 0.2).during(1.2)); // 振幅收缩
    tl.play(Anim::set_value(amp, 1.0).during(1.2)); // 振幅恢复
    tl.wait(0.6);

    // 装饰底条（无动画）
    tl.add(Item::rect(Rect::new(0.0, f64::from(h) - 8.0, f64::from(w), f64::from(h))).with_fill(accent));

    ffanim::render_to_mp4(&tl.compile(), "out/demo.mp4")?;
    println!("已输出 out/demo.mp4");
    Ok(())
}
