//! M8.4 验收示例：SVG 导入淡入 + 位图图片滑入缩放。
//!
//! 运行：`cargo run -p ffanim --example media`，产物 `out/media.mp4`。

use ffanim::kurbo::Affine;
use ffanim::{Anim, Color, Item, Timeline, from_svg_str};

const W: f64 = 960.0;
const H: f64 = 540.0;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bg = Color::new(14, 17, 22, 255);
    let mut tl = Timeline::new(W as u32, H as u32, 30, bg);

    // 1. SVG：三色徽标（两圆一矩形），整组淡入
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120">
        <circle cx="40" cy="60" r="28" fill="#e05d44"/>
        <circle cx="80" cy="60" r="28" fill="#f2b134"/>
        <rect x="52" y="52" width="16" height="16" fill="#3b88c3"/>
    </svg>"##;
    let logo = tl.add(from_svg_str(svg)?.with_translate(W / 2.0 - 60.0, 90.0));
    tl.play(Anim::fade_in(logo).during(1.0));

    // 2. 位图：程序生成 64x64 渐变方块，淡入后滑向左下并放大 3 倍
    let mut rgba = vec![0u8; 64 * 64 * 4];
    for y in 0..64u32 {
        for x in 0..64u32 {
            let i = ((y * 64 + x) * 4) as usize;
            rgba[i] = (x * 4) as u8;
            rgba[i + 1] = (y * 4) as u8;
            rgba[i + 2] = 180;
            rgba[i + 3] = 255;
        }
    }
    let img = tl.add(Item::from_image(rgba, 64, 64)?.with_translate(W / 2.0 - 32.0, H - 200.0));
    tl.play(Anim::fade_in(img).during(1.0));
    tl.play(
        Anim::transform_to(
            img,
            Affine::translate((W / 2.0 - 96.0, H - 264.0)) * Affine::scale(3.0),
        )
        .during(1.0),
    );

    let ct = tl.compile();
    ffanim::render_to_mp4(&ct, "out/media.mp4")?;
    println!("OK: out/media.mp4（{} 帧）", ct.duration_frames);
    Ok(())
}
