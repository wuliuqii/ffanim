//! 最小 ffanim 场景：圆形描边生长 → 底部装饰条 → 一步出片。
//!
//! 运行：`cargo run`（需要 PATH 里有 ffmpeg），产物 out/demo.mp4。

use ffanim::kurbo::{Circle, Rect};
use ffanim::{Anim, Color, Item, Timeline};

fn main() -> ffanim::Result<()> {
    let (w, h) = (960u32, 540u32);
    // 画布与颜色直接写死便于上手；正式项目建议像 coords 模板那样从主题 JSON 加载
    let bg = Color::new(14, 17, 22, 255);
    let accent = Color::new(120, 220, 232, 255);

    // 时间轴：构建期记录动作（两阶段模型，play 顺序即调度顺序）
    let mut tl = Timeline::new(w, h, 30, bg);

    // 圆形：描边生长（Create = 沿路径逐段显现）
    let cir = tl.add(
        Item::circle(Circle::new((f64::from(w) * 0.5, f64::from(h) * 0.55), 100.0))
            .with_stroke(accent)
            .with_stroke_width(3.0),
    );
    tl.play(Anim::create(cir).during(1.2));

    // 底部装饰条（无动画：全程可见）
    tl.add(Item::rect(Rect::new(0.0, f64::from(h) - 8.0, f64::from(w), f64::from(h))).with_fill(accent));

    tl.wait(0.5); // 尾部停留

    // 渲染：纯函数逐帧求值 + ffmpeg 管道编码
    ffanim::render_to_mp4(&tl.compile(), "out/demo.mp4")?;
    println!("已输出 out/demo.mp4");
    Ok(())
}
