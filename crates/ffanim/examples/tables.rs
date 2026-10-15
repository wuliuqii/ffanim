//! 表格场景族：core 侧网格拼装（TableSpec 三通道）+ 行组错峰入场 + 高亮底板标注。
//!
//! 对齐 manim `example_scenes/` 形态：单文件多场景 + argv 寻址。
//!
//! 运行（仓库根目录）：
//! `cargo run -p ffanim --release --example tables`（缺省全渲染，≙ `manim -a`）
//! `cargo run -p ffanim --example tables -- --json table_demo`（打印调度表，agent 检查用）

use ffanim::kurbo::Affine;
use ffanim::kurbo::Shape as _;
use ffanim::{Anim, Font, Item, Theme, Timeline};

const FPS: u32 = 30;

/// 场景清单（argv 寻址，缺省全渲染）。
const SCENES: &[&str] = &["table_demo"];

fn build(name: &str, theme: &Theme) -> ffanim::Result<Timeline> {
    match name {
        "table_demo" => table_demo(theme, FPS),
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

// ANCHOR: table_demo
/// M9.18 验收场景：P1-14 表格（manim `Table` 对应）。
///
/// 标题淡入 → 表格行组 `lagged_start` 逐行淡入 → 第二行高亮底板淡入
/// → `surrounding_rect` 圈注「示例」格（create 描边生长）。
/// 表格 = core 侧逐格排版拼组（行组保留独立动画句柄）；z 序 = 注册序，
/// 高亮底板须先于行组注册才能垫底。
fn table_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    let fg = theme.foreground();
    let accent = theme.accent();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background());

    // 表格文本/标题须 CJK 字形（find_cjk_font 优先，缺失时回退拉丁字体）
    let font = ffanim::typst::find_cjk_font()
        .and_then(|(p, idx)| ffanim::Font::from_index(&p, idx).ok())
        .map(Ok)
        .unwrap_or_else(|| Font::from_system(None::<&str>))?;

    // 1) 标题淡入（描边路径文本）
    let title_path = font.outline_text("复杂度对照表", 52.0)?;
    let bounds = title_path.bounding_box();
    let title = tl.add(Item::from_path(title_path).with_fill(fg).with_transform(
        Affine::translate((
            f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0,
            f64::from(h) * 0.10,
        )),
    ));
    tl.play(Anim::fade_in(title).during(0.7));
    tl.wait(0.2);

    // 2) 表格拼装（数据驱动：色皆出自主题，仅透明度派生）
    let o = ffanim::TableOptions {
        header_fill: Some(ffanim::Color::new(accent.r, accent.g, accent.b, 60)),
        ..ffanim::TableOptions::new(24.0, fg)
    };
    let spec = ffanim::table(
        &["运算", "复杂度", "示例"],
        &[
            vec!["求和", "O(n)", "sum(xs)"],
            vec!["排序", "O(n log n)", "sorted(xs)"],
        ],
        &font,
        &o,
    )?;
    // 表格从原点生长 → 平移到画布水平居中
    let b = spec.table.bbox();
    let move_t = Affine::translate((
        f64::from(w) / 2.0 - (b.x0 + b.x1) / 2.0,
        310.0 - (b.y0 + b.y1) / 2.0,
    ));

    // 高亮底板：先注册（z 序垫底），后播放
    let hl = tl.add(
        Item::rect(spec.row_rect(1))
            .with_transform(move_t)
            .with_fill(ffanim::Color::new(accent.r, accent.g, accent.b, 60)),
    );
    // 行组逐行注册 + 错峰淡入
    let row_ids: Vec<_> = spec
        .rows
        .iter()
        .map(|it| tl.add(it.clone().with_transform(move_t)))
        .collect();
    tl.lagged_start(row_ids.into_iter().map(Anim::fade_in), 0.5);
    tl.play(Anim::fade_in(hl).during(0.6));
    tl.wait(0.3);

    // 3) 圈注「示例」格：surrounding_rect 描边生长（未注册临时 Item 供定位）
    let ring = tl.add(
        Item::surrounding_rect(&Item::rect(spec.cell(1, 2)), 6.0)
            .with_transform(move_t)
            .with_stroke(accent),
    );
    tl.play(Anim::create(ring).during(0.6));
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END
