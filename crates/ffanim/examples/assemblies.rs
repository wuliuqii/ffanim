//! 装配件场景族：花括号 / 定界符 / 箭头 / 数字滚动（Typst MATH stretch 装配）。
//!
//! 对齐 manim `example_scenes/` 形态：单文件多场景 + argv 寻址。
//!
//! 运行（仓库根目录）：
//! `cargo run -p ffanim --release --example assemblies -- [场景名...]`（缺省全渲染，≙ `manim -a`）
//! `cargo run -p ffanim --example assemblies -- --json brace_demo`（打印调度表，agent 检查用）

use ffanim::kurbo::Shape as _;
use ffanim::kurbo::{Affine, Point, Rect};
use ffanim::{Anim, Font, Item, Theme, Timeline};

const FPS: u32 = 30;

/// 场景清单（argv 寻址，缺省全渲染）。
const SCENES: &[&str] = &["brace_demo", "delimiter_demo", "arrow_demo", "decimal_demo"];

fn build(name: &str, theme: &Theme) -> ffanim::Result<Timeline> {
    match name {
        "brace_demo" => brace_demo(theme, FPS),
        "delimiter_demo" => delimiter_demo(theme, FPS),
        "arrow_demo" => arrow_demo(theme, FPS),
        "decimal_demo" => decimal_demo(theme, FPS),
        other => Err(ffanim::Error::Other(format!(
            "未知场景 {other}，可用场景: {}",
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
    // 主题路径相对 crate 目录向上两级到仓库根（与 fourier.rs 同形态）
    let theme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes/default.json");
    let theme = Theme::from_path(theme_path)?;
    let _ = std::fs::create_dir_all("out");
    for name in &names {
        let tl = build(name, &theme)?.compile();
        if json {
            println!(
                "{}",
                serde_json::json!({
                    "scene": name,
                    "fps": tl.fps,
                    "duration_frames": tl.duration_frames,
                    "duration_seconds": tl.duration_frames as f64 / f64::from(tl.fps),
                    "items": tl.items.len(),
                    "anims": tl.anims.iter().map(|a| ffanim::AnimKind::describe(&a.kind)).collect::<Vec<_>>(),
                })
            );
        } else {
            ffanim::render_to_mp4(&tl, format!("out/{name}.mp4"))?;
            println!("{name}: {} 帧 → out/{name}.mp4", tl.duration_frames);
        }
    }
    Ok(())
}

// ANCHOR: brace_demo
/// M9.12 验收场景（P1-8）：Brace 花括号 + 括注文本。
///
/// 标题淡入 → 横条描边生长 → 下方花括号淡入放大 + 括注“宽度”淡入 →
/// 右侧花括号 + 括注“高度”（manim `Brace` / `brace.get_text` 对应）。
/// 花括号为填充字形，出场用 fade_in（manim `GrowFromCenter` 对应）而非 create。
fn brace_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Direction;
    use ffanim::typst::TypstEngine;

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim::typst::find_cjk_font()
        .and_then(|(path, idx)| ffanim::Font::from_index(&path, idx).ok())
        .map(Ok)
        .unwrap_or_else(|| Font::from_system(None::<&str>))?;
    // 花括号 = Typst MATH 装配（NewCMMath），与公式排版同引擎
    let mut engine = TypstEngine::new(8);
    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 1) 标题淡入
    let text_path = font.outline_text("Brace 括注", 56.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = tl.add(
        Item::from_path(text_path)
            .with_fill(fg)
            .with_transform(Affine::translate((dx, f64::from(h) * 0.10))),
    );
    tl.play(Anim::fade_in(title).during(0.7));

    // 2) 中央横条描边生长（留未注册副本作 brace 目标：bbox 只看几何）
    let bar_item = Item::rect(Rect::new(220.0, 200.0, 740.0, 310.0)).with_fill(accent);
    let bar = tl.add(bar_item.clone());
    tl.play(Anim::create(bar).during(1.0));

    // 3) 下方花括号 + 括注“宽度”
    let (brace_d, label_d) =
        engine.brace_with_text(&bar_item, Direction::Down, 14.0, &font, "宽度", 30.0)?;
    let (bd, ld) = (tl.add(brace_d.with_fill(fg)), tl.add(label_d.with_fill(fg)));
    tl.wait(0.2);
    tl.together(
        [Anim::fade_in(bd).during(1.0)]
            .into_iter()
            .chain([Anim::fade_in(ld).during(0.9)]),
    );

    // 4) 右侧花括号 + 括注“高度”
    let (brace_r, label_r) =
        engine.brace_with_text(&bar_item, Direction::Right, 14.0, &font, "高度", 30.0)?;
    let (br, lr) = (
        tl.add(brace_r.with_fill(accent)),
        tl.add(label_r.with_fill(accent)),
    );
    tl.wait(0.4);
    tl.together(
        [Anim::fade_in(br).during(1.0)]
            .into_iter()
            .chain([Anim::fade_in(lr).during(0.9)]),
    );

    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: delimiter_demo
/// M9.13 验收场景：Delimiter 四类定界符统一 Typst MATH 装配。
///
/// 标题淡入 → 横条生长 → 花括号↓ / 方括号↑ / 圆括号← / 双线方括号→
/// 依次淡入贴四侧 + 中文括注（manim `Brace`/`Bracket`/`Parenthesis`/
/// `DoubleBracket` 对应；四类共用同一装配核心，仅符号与深度比不同）。
fn delimiter_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Direction;
    use ffanim::typst::{Delimiter, TypstEngine};

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim::typst::find_cjk_font()
        .and_then(|(path, idx)| ffanim::Font::from_index(&path, idx).ok())
        .map(Ok)
        .unwrap_or_else(|| Font::from_system(None::<&str>))?;
    // 定界符 = Typst MATH 装配（NewCMMath GlyphAssembly），与公式排版同引擎
    let mut engine = TypstEngine::new(8);
    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 1) 标题淡入
    let text_path = font.outline_text("Delimiter 定界符", 56.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = tl.add(
        Item::from_path(text_path)
            .with_fill(fg)
            .with_transform(Affine::translate((dx, f64::from(h) * 0.10))),
    );
    tl.play(Anim::fade_in(title).during(0.7));

    // 2) 中央横条描边生长（留未注册副本作定界符目标：bbox 只看几何）
    let bar_item = Item::rect(Rect::new(280.0, 210.0, 680.0, 310.0)).with_fill(accent);
    let bar = tl.add(bar_item.clone());
    tl.play(Anim::create(bar).during(1.0));

    // 3) 四类定界符依次贴四侧 + 括注（交替前景/强调色；每类同窗淡入）
    let specs: [(Delimiter, Direction, &str); 4] = [
        (Delimiter::Brace, Direction::Down, "宽度"),
        (Delimiter::Bracket, Direction::Up, "上沿"),
        (Delimiter::Paren, Direction::Left, "左沿"),
        (Delimiter::DoubleBracket, Direction::Right, "右沿"),
    ];
    for (i, (kind, dir, label)) in specs.into_iter().enumerate() {
        let (delim, text) =
            engine.delimiter_with_text(&bar_item, kind, dir, 14.0, &font, label, 30.0)?;
        let color = if i % 2 == 0 { fg } else { accent };
        let (d, t) = (
            tl.add(delim.with_fill(color)),
            tl.add(text.with_fill(color)),
        );
        tl.wait(0.15);
        tl.together(
            [Anim::fade_in(d).during(0.6)]
                .into_iter()
                .chain([Anim::fade_in(t).during(0.5)]),
        );
    }

    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: arrow_demo
/// arrow_demo：Typst 装配箭头验收示例（M9.14）。
///
/// 标题淡入 → 三支箭头（水平单头 / 斜向单头 / 水平双头）依次淡入 +
/// 中文括注（manim `Arrow`/`DoubleArrow` 对应；头高随跨度自适应并封顶，
/// 双头两端对称）。形状 = Typst MATH stretch 装配（NewCMMath GlyphAssembly），
/// 闭合填充字形，出场用 fade_in（填充字形禁用 create）。
fn arrow_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{ArrowKind, TypstEngine};

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim::typst::find_cjk_font()
        .and_then(|(path, idx)| ffanim::Font::from_index(&path, idx).ok())
        .map(Ok)
        .unwrap_or_else(|| Font::from_system(None::<&str>))?;
    // 箭头 = Typst MATH stretch 装配，与公式排版同引擎
    let mut engine = TypstEngine::new(8);
    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 1) 标题淡入
    let text_path = font.outline_text("Arrow 箭头", 56.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = tl.add(
        Item::from_path(text_path)
            .with_fill(fg)
            .with_transform(Affine::translate((dx, f64::from(h) * 0.10))),
    );
    tl.play(Anim::fade_in(title).during(0.7));

    // 2) 三支箭头依次淡入 + 括注（交替前景/强调色；括注在杆中点上方居中）
    let specs: [(ArrowKind, Point, Point, &str); 3] = [
        (
            ArrowKind::Single,
            Point::new(220.0, 300.0),
            Point::new(740.0, 300.0),
            "单头",
        ),
        (
            ArrowKind::Single,
            Point::new(280.0, 460.0),
            Point::new(680.0, 380.0),
            "斜向",
        ),
        (
            ArrowKind::Double,
            Point::new(300.0, 200.0),
            Point::new(660.0, 200.0),
            "双头",
        ),
    ];
    for (i, (kind, from, to, label)) in specs.into_iter().enumerate() {
        let arrow = engine.arrow(from, to, kind)?;
        // 括注：杆中点上方 34px，bbox 中心对齐杆中点
        let mid = Point::new((from.x + to.x) / 2.0, (from.y + to.y) / 2.0);
        let label_path = font.outline_text(label, 30.0).unwrap();
        let lb = label_path.bounding_box();
        let lx = mid.x - (lb.x0 + lb.x1) / 2.0;
        let ly = mid.y - 34.0 - (lb.y0 + lb.y1) / 2.0;
        let color = if i % 2 == 0 { fg } else { accent };
        let (a, t) = (
            tl.add(arrow.with_fill(color)),
            tl.add(
                Item::from_path(label_path)
                    .with_fill(color)
                    .with_transform(Affine::translate((lx, ly))),
            ),
        );
        tl.wait(0.15);
        tl.together(
            [Anim::fade_in(a).during(0.8)]
                .into_iter()
                .chain([Anim::fade_in(t).during(0.6)]),
        );
    }

    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: decimal_demo
/// M9.15 验收场景：P1-6 数字滚动（manim `DecimalNumber` + ValueTracker 对应）。
///
/// 标题淡入 → 主数字 0→1,234.5 千分位滚动（中心锚定不漂移）→ 副数字
/// 带符号 -40→+40 滚动。数字 = Typst markup 逐帧排版（闭合填充字形，
/// 出场 fade_in 禁 create）；tracker 值动画驱动，updater 每帧重建形状。
fn decimal_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{DecimalNumberOptions, TypstEngine};

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim::typst::find_cjk_font()
        .and_then(|(path, idx)| ffanim::Font::from_index(&path, idx).ok())
        .map(Ok)
        .unwrap_or_else(|| Font::from_system(None::<&str>))?;
    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 1) 标题淡入
    let text_path = font.outline_text("Decimal 数字滚动", 56.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = tl.add(
        Item::from_path(text_path)
            .with_fill(fg)
            .with_transform(Affine::translate((dx, f64::from(h) * 0.10))),
    );
    tl.play(Anim::fade_in(title).during(0.7));

    // 2) 主数字：0 → 1,234.5 千分位滚动（中心锚定，updater 每帧重排）
    let anchor = Point::new(f64::from(w) / 2.0, 250.0);
    let sig = TypstEngine::decimal_signal(
        anchor,
        DecimalNumberOptions {
            decimals: 1,
            group_with_commas: true,
            include_sign: false,
            font_size_pt: 36.0,
            text_color: fg,
        },
    )?;
    let t = tl.add_tracker(0.0);
    let num = tl.add_updater(&[t], sig);
    tl.play(Anim::fade_in(num).during(0.6));
    tl.play(Anim::set_value(t, 1234.5).during(2.5));
    tl.wait(0.5);

    // 3) 副数字：-40 → +40 带符号滚动（include_sign：正数显 +）
    let sig2 = TypstEngine::decimal_signal(
        Point::new(f64::from(w) / 2.0, 380.0),
        DecimalNumberOptions {
            decimals: 0,
            include_sign: true,
            group_with_commas: false,
            font_size_pt: 28.0,
            text_color: accent,
        },
    )?;
    let t2 = tl.add_tracker(-40.0);
    let num2 = tl.add_updater(&[t2], sig2);
    tl.play(Anim::fade_in(num2).during(0.6));
    tl.play(Anim::set_value(t2, 40.0).during(2.0));
    tl.wait(0.6);
    Ok(tl)
}
// ANCHOR_END
