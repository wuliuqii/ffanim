//! 分课教程素材：六课场景（每课一个函数 ≈30 行，正文逐段讲解见示例书 Part III）。
//!
//! gally 中登记为素材文件（Asset）：只出片 + 拷媒体，不生成画廊章节页；
//! 各课函数体以 `// ANCHOR: <课名>` 标注，供 lesson-*.md 手写页 `{{#include}}`。
//!
//! 运行（仓库根目录）：
//! `cargo run --release -p ffanim --example lessons -- [课名...]`（缺省全渲染）
//! 产物：out/<课名>.mp4（对应示例书第 1~6 课的视频）。

use ffanim::kurbo::Affine;
use ffanim::kurbo::{Circle, Rect, Vec2};
use ffanim::{Anim, Item, Theme, Timeline};

const FPS: u32 = 30;

/// 场景清单（argv 寻址，缺省全渲染；顺序 = 示例书课程顺序）。
const SCENES: &[&str] = &[
    "lesson_shapes",
    "lesson_anims",
    "lesson_text",
    "lesson_axes",
    "lesson_layout",
    "lesson_camera",
];

fn build(name: &str, theme: &Theme) -> ffanim::Result<Timeline> {
    match name {
        "lesson_shapes" => lesson_shapes(theme, FPS),
        "lesson_anims" => lesson_anims(theme, FPS),
        "lesson_text" => lesson_text(theme, FPS),
        "lesson_axes" => lesson_axes(theme, FPS),
        "lesson_layout" => lesson_layout(theme, FPS),
        "lesson_camera" => lesson_camera(theme, FPS),
        other => Err(ffanim::Error::Other(format!(
            "未知课名 {other}，可用课程: {}",
            SCENES.join(", ")
        ))),
    }
}

fn main() -> ffanim::Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.first().is_some_and(|a| a == "--json");
    if json {
        args.remove(0);
    }
    let names: Vec<String> = if args.is_empty() {
        SCENES.iter().map(|s| (*s).to_string()).collect()
    } else {
        args
    };
    let theme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes/default.json");
    let theme = Theme::from_path(theme_path)?;
    let _ = std::fs::create_dir_all("out");
    for name in &names {
        let tl = build(name, &theme)?.compile();
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "lesson": name,
                    "duration_seconds": tl.duration_frames as f64 / f64::from(tl.fps),
                })
            );
        } else {
            ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
            println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
        }
    }
    Ok(())
}

// ANCHOR: lesson_shapes
/// 第 1 课 · 形状与样式：三种基元 × 三种着色（填充 / 纯描边 / 半透明）。
fn lesson_shapes(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    let accent = theme.accent();
    let fg = theme.foreground();
    let mut tl = Timeline::new(960, 540, fps, theme.background());

    // 圆：填充 + 描边（fill 决定内部色，stroke 决定轮廓色与粗细）
    let circle = tl.add(
        Item::circle(Circle::new((200.0, 270.0), 90.0))
            .with_fill(accent)
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    // 矩形：只描边不填充（轮廓线框）
    let rect = tl.add(
        Item::rect(Rect::new(370.0, 180.0, 590.0, 360.0))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    // 六边形：半透明填充（with_alpha 调透明度，0 全透 / 255 不透）
    let hex = tl.add(
        Item::regular_polygon((760.0, 270.0).into(), 90.0, 6)
            .with_fill(accent.with_alpha(90))
            .with_stroke(accent)
            .with_stroke_width(3.0),
    );
    // 三件一起描边生长（group 错峰 0.15s）
    tl.group(
        [circle, rect, hex].map(|id| Anim::create(id).during(0.9)),
        0.15,
    );
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: lesson_anims
/// 第 2 课 · 动画词汇：一条时间线串起最常用的四个动画段。
fn lesson_anims(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    let fg = theme.foreground();
    let accent = theme.accent();
    let mut tl = Timeline::new(960, 540, fps, theme.background());

    // 1) create：描边像手绘一样「生长」出来（during 定时长，rate 定缓动）
    let shape = tl.add(
        Item::rect(Rect::new(370.0, 165.0, 590.0, 375.0))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    tl.play(Anim::create(shape).during(1.0));
    // 2) transform：注册目标圆，矩形逐点变形为圆（真正意义的形状渐变）
    let circle = tl.add(
        Item::circle(Circle::new((480.0, 270.0), 105.0))
            .with_fill(accent)
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    tl.wait(0.3);
    tl.play(Anim::transform(shape, circle).during(1.2));
    // 3) move_to：绝对平移到画布中心（还有 shift_to/rotate_to/scale_to 相对版）
    tl.wait(0.3);
    tl.play(Anim::move_to(shape, (480.0, 270.0)).during(0.9));
    // 4) fade_out：淡出退场（默认 rate=Smooth，可换 Linear/EaseIn…）
    tl.play(Anim::fade_out(shape).during(0.7));
    tl.wait(0.4);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: lesson_text
/// 第 3 课 · 文字与公式：Typst 排版公式与中文文本（CJK 字体注入）。
fn lesson_text(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{FormulaOptions, TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground();
    let accent = theme.accent();
    let mut tl = Timeline::new(960, 540, fps, theme.background());

    // 排版引擎 + 系统中文字体（中文文本必须有 CJK 字形，否则口占位）
    let mut engine = TypstEngine::new(16);
    if let Some((font, idx)) = find_cjk_font() {
        engine.add_font_face(&font, idx)?;
    }
    // 公式：Typst math 语法；to_items() 拆散件，fit_transform 落位（scale 2.0 = pt→px）
    let fopts = FormulaOptions {
        font_size_pt: 32.0,
        text_color: fg,
    };
    let formula = engine.formula("e^{i pi} + 1 = 0", &fopts)?;
    let ids: Vec<_> = formula
        .to_items()
        .into_iter()
        .map(|it| tl.add(it.with_transform(formula.fit_transform(480.0, 200.0, 2.0))))
        .collect();
    tl.group(ids.iter().map(|&id| Anim::create(id).during(0.5)), 0.08); // 逐字形写出
    // 文本：Typst markup 语法（*…* 粗体）；闭合填充字形用 fade_in（禁 create）
    let text = engine.text(
        "欧拉公式：*数学里最优美的等式*",
        &TextOptions::new(22.0, accent),
    )?;
    let tids: Vec<_> = text
        .to_items()
        .into_iter()
        .map(|it| tl.add(it.with_transform(text.fit_transform(480.0, 340.0, 2.0))))
        .collect();
    tl.wait(0.3);
    tl.group(tids.iter().map(|&id| Anim::fade_in(id).during(0.8)), 0.0);
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: lesson_axes
/// 第 4 课 · 坐标系与函数图像：Axes.plot 画曲线 + tracker 驱动动点巡行。
fn lesson_axes(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::{Axes, RateFunc};

    let accent = theme.accent();
    let fg = theme.foreground();
    let mut tl = Timeline::new(960, 540, fps, theme.background());

    // 坐标系：数据范围 x∈[-4,4] y∈[-1.6,1.6]，占画布 800×360 居中
    let axes = Axes::new((-4.0, 4.0), (-1.6, 1.6))
        .with_size(800.0, 360.0)
        .with_center(480.0, 270.0);
    let ax = tl.add(axes.to_item(&theme.style));
    tl.play(Anim::create(ax).during(1.0));
    // 函数图像：plot 直接吃闭包，c2p 负责数据坐标 → 画布像素
    let f = |x: f64| 1.2 * (1.3 * x).sin();
    let curve = tl.add(
        axes.plot(f, (-4.0, 4.0))
            .with_stroke(accent)
            .with_stroke_width(3.0),
    );
    tl.play(Anim::create(curve).during(1.2));
    // 动点：tracker 匀速推进 0→1，updater 每帧重建圆的位置（渲染期查表求值）
    let t = tl.add_tracker(0.0);
    let _dot = tl.add_updater(&[t], move |v| {
        let x = -4.0 + 8.0 * v[0];
        Item::circle(Circle::new(axes.c2p(x, f(x)), 9.0)).with_fill(fg)
    });
    tl.play(Anim::set_value(t, 1.0).during(3.0).rate(RateFunc::Linear));
    tl.wait(0.5);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: lesson_layout
/// 第 5 课 · 布局与强调：arrange 排队布局 + indicate / circumscribe 强调。
fn lesson_layout(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{FormulaOptions, TypstEngine};
    use ffanim::{Direction, arrange};

    let fg = theme.foreground();
    let accent = theme.accent();
    let mut tl = Timeline::new(960, 540, fps, theme.background());
    let mut engine = TypstEngine::new(16);
    let fopts = FormulaOptions {
        font_size_pt: 26.0,
        text_color: fg,
    };

    // 三个公式各自拼成整块（group），竖排排队（arrange Down 返回总包围盒）
    let mut blocks: Vec<Item> = ["a^2 + b^2 = c^2", "E = m c^2", "e^{i pi} + 1 = 0"]
        .iter()
        .map(|body| {
            let f = engine.formula(body, &fopts).expect("排版公式");
            let place = f.center_transform(0.0, 0.0); // 先都聚到原点，排队后统一挪
            Item::group(
                f.to_items()
                    .into_iter()
                    .map(|it| it.with_transform(place))
                    .collect(),
            )
        })
        .collect();
    let total = arrange(&mut blocks, Direction::Down, 50.0);
    let shift = Affine::translate(Vec2::new(
        480.0 - total.center().x,
        270.0 - total.center().y,
    ));
    let ids: Vec<_> = blocks
        .into_iter()
        .map(|mut it| {
            it.transform = shift * it.transform;
            tl.add(it)
        })
        .collect();
    tl.group(ids.iter().map(|&id| Anim::fade_in(id).during(0.7)), 0.2);
    tl.wait(0.3);
    // 强调两件套：indicate 轻晃提醒「注意这项」，circumscribe 圈框锁定
    tl.indicate(ids[1], 1.08, 0.6);
    tl.wait(0.3);
    tl.circumscribe(ids[2], 1.0, 14.0, accent);
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: lesson_camera
/// 第 6 课 · 镜头语言：相机推近注视曲线峰顶，再拉回全景。
fn lesson_camera(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::{Axes, Camera};

    let accent = theme.accent();
    let mut tl = Timeline::new(960, 540, fps, theme.background());

    // 场景内容：一条正弦曲线（推近后细节更清晰）
    let axes = Axes::new((-4.0, 4.0), (-1.6, 1.6))
        .with_size(880.0, 360.0)
        .with_center(480.0, 270.0);
    let ax = tl.add(axes.to_item(&theme.style));
    let curve = tl.add(
        axes.plot(|x| 1.2 * (1.3 * x).sin(), (-4.0, 4.0))
            .with_stroke(accent)
            .with_stroke_width(3.0),
    );
    tl.group([ax, curve].map(|id| Anim::create(id).during(0.9)), 0.1);
    // 相机：注册为时间轴对象；transform_to 推近 = 注视点映射到屏心 + zoom 放大
    let camera = tl.camera(Camera::at(960, 540));
    let peak_x = 1.2_f64; // 曲线峰顶（f 峰值处）
    let peak = axes.c2p(peak_x, 1.2 * (1.3 * peak_x).sin());
    tl.play(
        Anim::transform_to(
            camera,
            Camera {
                center: Vec2::new(peak.x, peak.y),
                zoom: 1.8,
                rotation: 0.0,
            }
            .affine(960, 540),
        )
        .during(1.2),
    );
    tl.wait(0.6);
    // 拉回全景收尾
    tl.play(Anim::transform_to(camera, Camera::at(960, 540).affine(960, 540)).during(1.0));
    tl.wait(0.5);
    Ok(tl)
}
// ANCHOR_END
