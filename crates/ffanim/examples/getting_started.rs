//! ffanim 入门示例：矩形描边生长 → 平滑变形为圆 → 淡出。
//!
//! 全书最短的一条动画，只用三个概念——Item（画什么）、Anim（怎么变）、
//! Timeline（何时变）。运行（仓库根目录）：
//! `cargo run --release -p ffanim --example getting_started`
//! 产物：out/getting_started.mp4（约 2.5 秒）。

use ffanim::kurbo::{Circle, Rect};
use ffanim::{Anim, Item, Theme, Timeline};

fn main() -> ffanim::Result<()> {
    // ANCHOR: setup
    // 主题来自 JSON（颜色/描边宽度零硬编码）；时间轴 = 960×540@30fps
    let theme = Theme::from_path(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../themes/default.json"
    ))?;
    let mut tl = Timeline::new(960, 540, 30, theme.background().unwrap());
    let fg = theme.foreground().unwrap();
    let _ = std::fs::create_dir_all("out");
    // ANCHOR_END: setup

    // ANCHOR: create_item
    // 1) 注册矩形，create 让描边像手绘一样「生长」出来
    let square = tl.add(
        Item::rect(Rect::new(370.0, 165.0, 590.0, 375.0))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    tl.play(Anim::create(square).during(1.0));
    // ANCHOR_END: create_item

    // ANCHOR: morph
    // 2) 注册目标圆形，transform 逐点把矩形变形为圆
    let circle = tl.add(
        Item::circle(Circle::new((480.0, 270.0), 105.0))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    tl.play(Anim::transform(square, circle).during(1.0));
    // ANCHOR_END: morph

    // ANCHOR: finish
    // 3) 淡出收尾
    tl.play(Anim::fade_out(square).during(0.5));

    // 编译时间轴（构建期记录 → 求值期查表）并渲染成 MP4
    ffanim::render_to_mp4(&tl.compile(), "out/getting_started.mp4")
    // ANCHOR_END: finish
}
