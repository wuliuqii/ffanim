//! 基础场景族：动画词汇 / 正弦波 / 几何基元 / 布局 / 强调 / 语法糖 / 移动 / 形状变换。
//!
//! 对齐 manim `example_scenes/basic.py` 形态：单文件多场景 + argv 寻址。
//!
//! 运行（仓库根目录）：
//! `cargo run -p ffanim --release --example basic -- [场景名...]`（缺省全渲染，≙ `manim -a`）
//! `cargo run -p ffanim --example basic -- --json sine_demo`（打印调度表，agent 检查用）

use ffanim::kurbo::Shape as _;
use ffanim::kurbo::{Affine, BezPath, Circle, Point, Rect};
use ffanim::{Anim, Font, Item, Theme, Timeline};

const FPS: u32 = 30;

/// 场景清单（argv 寻址，缺省全渲染）。
const SCENES: &[&str] = &[
    "demo_anim",
    "sine_demo",
    "primitives_demo",
    "layout_demo",
    "emphasize_demo",
    "sugar_demo",
    "move_demo",
    "shapes_demo",
];

fn build(name: &str, theme: &Theme, font: &Font) -> ffanim::Result<Timeline> {
    match name {
        "demo_anim" => Ok(demo_anim(theme, font, FPS)),
        "sine_demo" => sine_demo(theme, FPS),
        "primitives_demo" => Ok(primitives_demo(theme, FPS)),
        "layout_demo" => Ok(layout_demo(theme, FPS)),
        "emphasize_demo" => Ok(emphasize_demo(theme, FPS)),
        "sugar_demo" => Ok(sugar_demo(theme, FPS)),
        "move_demo" => move_demo(theme, FPS),
        "shapes_demo" => shapes_demo(theme, FPS),
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
    let font = Font::from_system(None::<&str>)?;
    let _ = std::fs::create_dir_all("out");
    for name in &names {
        let tl = build(name, &theme, &font)?.compile();
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

// ANCHOR: demo_anim
/// M2 验收场景：三角形描边生长 → 圆形滑入 → 圆形形变放大 → 文字淡入。
fn demo_anim(theme: &Theme, font: &Font, fps: u32) -> Timeline {
    let accent = theme.accent().unwrap();
    let foreground = theme.foreground().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960u32, 540u32);

    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 三角形：左侧，描边生长
    let tri = Item::regular_polygon((f64::from(w) * 0.25, f64::from(h) * 0.62).into(), 120.0, 3)
        .with_stroke(accent)
        .with_stroke_width(sw);
    let tri = tl.add(tri);
    tl.play(Anim::create(tri).during(1.2));

    // 圆形：右侧，从下方滑入淡入
    let cir_center: ffanim::kurbo::Point = (f64::from(w) * 0.75, f64::from(h) * 0.62).into();
    let circle = Item::circle(Circle::new(cir_center, 100.0))
        .with_fill(accent)
        .with_stroke(foreground)
        .with_stroke_width(sw);
    let circle = tl.add(circle);
    tl.wait(0.3);
    tl.play(Anim::fade_in(circle).during(0.8).shift(0.0, 80.0));

    // 圆形形变：放大并变色（同为 4 段三次贝塞尔，逐点形变）
    let circle_big = Item::circle(Circle::new(cir_center, 160.0))
        .with_fill(foreground)
        .with_stroke(accent)
        .with_stroke_width(sw);
    let circle_big = tl.add(circle_big);
    tl.wait(0.4);
    tl.play(Anim::transform(circle, circle_big).during(1.2));

    // 标题文字：顶部居中淡入
    let text_path = font.outline_text("ffanim M2", 72.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = Item::from_path(text_path)
        .with_fill(foreground)
        .with_transform(Affine::translate((dx, f64::from(h) * 0.15)));
    let title = tl.add(title);
    tl.wait(0.3);
    tl.play(Anim::fade_in(title).during(0.8));

    // 装饰底条（无动画：全程可见）
    let bar = Item::rect(Rect::new(
        0.0,
        f64::from(h) - 8.0,
        f64::from(w),
        f64::from(h),
    ))
    .with_fill(accent);
    tl.add(bar);

    tl.wait(0.8);
    tl
}
// ANCHOR_END

// ANCHOR: sine_demo
/// M4 验收场景：坐标系 + 正弦函数图像随 ValueTracker 动画（求值期信号）。
///
/// 网格淡入 → 坐标轴描边生长 → sin 曲线逐段画出 → 振幅 1.0→0.2→1.0 呼吸。
fn sine_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Axes;

    let accent = theme.accent().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let axes = Axes::new((-3.0, 3.0), (-1.2, 1.2))
        .with_size(840.0, 400.0)
        .with_center(f64::from(w) / 2.0, f64::from(h) / 2.0);

    // 网格先淡入，坐标轴描边生长
    let grid = tl.add(axes.grid_item(theme.foreground().unwrap().with_alpha(40), 1.0));
    tl.play(Anim::fade_in(grid).during(0.6));
    let ax = tl.add(axes.to_item(&theme.style));
    tl.overlap(0.4);
    tl.play(Anim::create(ax).during(1.0));

    // 正弦函数图像：振幅由 ValueTracker 驱动（渲染期每帧重建）
    let amp = tl.add_tracker(1.0);
    let graph = tl.add_updater(&[amp], move |v| {
        axes.plot(
            |x| v[0] * (x * std::f64::consts::FRAC_PI_2).sin(),
            (-3.0, 3.0),
        )
        .with_stroke(accent)
        .with_stroke_width(3.0)
    });
    tl.wait(0.2);
    tl.play(Anim::create(graph).during(1.2));
    tl.wait(0.3);

    // 振幅呼吸
    tl.play(Anim::set_value(amp, 0.2).during(1.2));
    tl.play(Anim::set_value(amp, 1.0).during(1.2));
    tl.wait(0.6);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: primitives_demo
/// manim 对齐 P0 验收场景：标注基元（线/箭头/虚线/弧/椭圆）+ 位姿动作糖。
///
/// 前半段基元逐个描边生长（group 错开），后半段演示 `shift_to / rotate_to /
/// scale_to / move_to` 四个相对位姿动作（构建期解析为绝对 TransformTo）。
fn primitives_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim::typst::{ArrowKind, TypstEngine};
    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960.0_f64, 540.0_f64);
    let mut tl = Timeline::new(w as u32, h as u32, fps, theme.background().unwrap());

    // —— 标注基元逐个 create（group 错开 0.15s）——
    let line = tl.add(
        Item::line((100.0, 120.0), (300.0, 120.0))
            .with_stroke(fg)
            .with_stroke_width(sw),
    );
    // 箭头 = Typst MATH stretch 装配（闭合填充字形，with_fill 着色）
    let mut engine = TypstEngine::new(8);
    let arrow = tl.add(
        engine
            .arrow(
                Point::new(380.0, 120.0),
                Point::new(560.0, 120.0),
                ArrowKind::Single,
            )
            .expect("装配箭头")
            .with_fill(accent),
    );
    let dash = tl.add(
        Item::dashed_line((640.0, 120.0), (860.0, 120.0), 14.0, 10.0)
            .with_stroke(fg)
            .with_stroke_width(sw),
    );
    let arc = tl.add(
        Item::arc((200.0, 360.0), 80.0, 0.0, 180.0)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    let ell = tl.add(
        Item::ellipse((620.0, 360.0), 140.0, 70.0)
            .with_stroke(fg)
            .with_stroke_width(sw),
    );
    // 基元描边生长 create；装配箭头为填充字形，单独 fade_in（禁 create）
    let ids = [line, dash, arc, ell];
    tl.group(ids.iter().map(|&id| Anim::create(id).during(0.8)), 0.15);
    tl.play(Anim::fade_in(arrow).during(0.8));
    tl.wait(0.4);

    // —— 位姿动作糖：相对目标在 play 时按基线解析为绝对 TransformTo ——
    tl.play(Anim::shift_to(arrow, 80.0, 0.0).during(0.8)); // 箭头右移
    tl.overlap(0.4);
    tl.play(Anim::rotate_to(arc, std::f64::consts::FRAC_PI_2).during(0.8)); // 弧视觉逆时针转 90°
    tl.overlap(0.4);
    tl.play(Anim::scale_to(ell, 1.4).during(0.8)); // 椭圆放大
    tl.wait(0.2);
    tl.play(Anim::move_to(ell, (w / 2.0, h / 2.0)).during(1.0)); // 移到画布中心
    tl.wait(0.6);
    tl
}
// ANCHOR_END

// ANCHOR: layout_demo
/// manim 对齐 P0-2 验收场景：相对布局 next_to / to_edge / arrange。
///
/// 标题块贴顶边（to_edge）→ 三基元横排居中（arrange + 总包围盒平移）→
/// 步骤列表竖排（arrange）；全部为构建期布局，动画只做淡入/描边生长。
fn layout_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim::kurbo::Vec2;
    use ffanim::{Direction, ItemId, arrange};

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960.0_f64, 540.0_f64);
    let mut tl = Timeline::new(w as u32, h as u32, fps, theme.background().unwrap());

    // 标题块贴顶边（to_edge：另一轴对画布居中）
    let title = tl.add(
        Item::rect(Rect::new(0.0, 0.0, 360.0, 56.0))
            .with_fill(accent)
            .to_edge(Direction::Up, 24.0, (w, h)),
    );
    tl.play(Anim::fade_in(title).during(0.6));

    // 三基元横排（arrange Right），总包围盒平移到画布中心
    let c = Item::circle(Circle::new((0.0, 0.0), 60.0))
        .with_stroke(fg)
        .with_stroke_width(sw);
    let r = Item::rect(Rect::new(0.0, 0.0, 140.0, 90.0))
        .with_stroke(fg)
        .with_stroke_width(sw);
    let e = Item::ellipse((0.0, 0.0), 90.0, 45.0)
        .with_stroke(fg)
        .with_stroke_width(sw);
    let mut row = vec![c, r, e];
    let total = arrange(&mut row, Direction::Right, 40.0);
    let shift = Affine::translate(Vec2::new(
        w / 2.0 - total.center().x,
        h / 2.0 - total.center().y,
    ));
    let row_ids: Vec<ItemId> = row
        .into_iter()
        .map(|mut it| {
            let t = shift * it.transform;
            it.transform = t;
            tl.add(it)
        })
        .collect();
    tl.group(row_ids.iter().map(|&id| Anim::create(id).during(0.7)), 0.15);
    tl.wait(0.2);

    // 步骤列表竖排（arrange Down），平移到画布左侧
    let mut col: Vec<Item> = (0..4)
        .map(|_| {
            Item::rect(Rect::new(0.0, 0.0, 120.0, 44.0))
                .with_stroke(fg)
                .with_stroke_width(sw)
        })
        .collect();
    arrange(&mut col, Direction::Down, 20.0);
    let to_left = Affine::translate(Vec2::new(80.0, 170.0));
    let col_ids: Vec<ItemId> = col
        .into_iter()
        .map(|mut it| {
            let t = to_left * it.transform;
            it.transform = t;
            tl.add(it)
        })
        .collect();
    tl.group(col_ids.iter().map(|&id| Anim::fade_in(id).during(0.5)), 0.1);
    tl.wait(0.6);
    tl
}
// ANCHOR_END

// ANCHOR: emphasize_demo
/// M9.5 验收场景：强调四件套（indicate / circumscribe / flash / show_passing_flash）。
///
/// 对应 manim 的 Indicate / Circumscribe / Flash / ShowPassingFlash；
/// 全部为构建期编排（两段/三段既有词汇），求值路径零改动。
fn emphasize_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim::kurbo::BezPath;

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960.0_f64, 540.0_f64);
    let mut tl = Timeline::new(w as u32, h as u32, fps, theme.background().unwrap());

    // 左上：圆 create 后 indicate（放大 1.3 倍再复原——"注意这一项"）
    let circle = tl.add(
        Item::circle(Circle::new((220.0, 160.0), 70.0))
            .with_stroke(fg)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(circle).during(0.8));
    tl.wait(0.2);
    tl.indicate(circle, 1.3, 0.8);

    // 右上：矩形 create 后 circumscribe（外框淡入淡出圈选）
    let rect = tl.add(
        Item::rect(Rect::new(0.0, 0.0, 200.0, 140.0))
            .with_stroke(fg)
            .with_stroke_width(sw)
            .with_transform(Affine::translate((580.0, 90.0))),
    );
    tl.play(Anim::create(rect).during(0.8));
    tl.circumscribe(rect, 0.8, 20.0, accent);

    // 中下：flash（扩散圆环一闪，指向空白处"这里"）
    tl.flash((w / 2.0, h * 0.62), 0.6, 30.0, accent);
    tl.wait(0.3);

    // 底部折线：create 后沿路径中段 passing flash（局部高亮描边）
    let mut zig = BezPath::new();
    zig.move_to((180.0, 430.0));
    zig.line_to((380.0, 360.0));
    zig.line_to((580.0, 460.0));
    zig.line_to((780.0, 400.0));
    let zig_id = tl.add(Item::from_path(zig).with_stroke(fg).with_stroke_width(sw));
    tl.play(Anim::create(zig_id).during(0.8));
    tl.show_passing_flash(zig_id, 0.25, 0.75, 0.8, accent);
    tl
}
// ANCHOR_END

// ANCHOR: sugar_demo
/// M9.7 验收场景：P1 薄糖批（Uncreate / Together / Succession / 方向家族 /
/// add_group / save_state-restore）。
fn sugar_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim::Direction;
    use ffanim::typst::{ArrowKind, TypstEngine};

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960.0_f64, 540.0_f64);
    let mut tl = Timeline::new(w as u32, h as u32, fps, theme.background().unwrap());

    // 1) Uncreate：圆描边生长 → 收尾收回（create 的镜像）
    let circle = tl.add(
        Item::circle(Circle::new((200.0, 140.0), 60.0))
            .with_stroke(fg)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(circle).during(0.8));
    tl.wait(0.2);
    tl.play(Anim::uncreate(circle).during(0.8));

    // 2) Together：方块与圆同窗淡入（时长不同，游标取最长）
    let rect_a = tl.add(
        Item::rect(Rect::new(0.0, 0.0, 130.0, 80.0))
            .with_stroke(fg)
            .with_stroke_width(sw)
            .with_transform(Affine::translate((560.0, 70.0))),
    );
    let circle_b = tl.add(
        Item::circle(Circle::new((830.0, 120.0), 45.0))
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.together([
        Anim::fade_in(rect_a).during(0.7),
        Anim::fade_in(circle_b).during(1.0),
    ]);

    // 3) Succession：三支箭头依次淡入（装配字形禁 create）
    // 箭头 = Typst MATH stretch 装配（闭合填充字形，with_fill 着色）
    let mut engine = TypstEngine::new(8);
    let arrows: Vec<Item> = (0..3)
        .map(|i: i32| {
            let x = f64::from(i) * 200.0;
            engine
                .arrow(
                    Point::new(150.0 + x, 280.0),
                    Point::new(260.0 + x, 280.0),
                    ArrowKind::Single,
                )
                .expect("装配箭头")
                .with_fill(fg)
        })
        .collect();
    let arrow_ids = tl.add_group(arrows);
    tl.succession([
        Anim::fade_in(arrow_ids[0]).during(0.4),
        Anim::fade_in(arrow_ids[1]).during(0.4),
        Anim::fade_in(arrow_ids[2]).during(0.4),
    ]);

    // 4) 方向家族：中心长出菱形 → 旋转进入三角 → 从左滑入方块
    let diamond = tl.add(
        Item::regular_polygon((700.0, 350.0).into(), 50.0, 4)
            .with_stroke(fg)
            .with_stroke_width(sw),
    );
    tl.play(Anim::grow_from_center(diamond).during(0.6));
    let tri = tl.add(
        Item::regular_polygon((855.0, 350.0).into(), 45.0, 3)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.play(Anim::spin_in(tri, std::f64::consts::PI).during(0.6));
    let slider = tl.add(
        Item::rect(Rect::new(0.0, 0.0, 110.0, 70.0))
            .with_stroke(fg)
            .with_stroke_width(sw)
            .with_transform(Affine::translate((120.0, 380.0))),
    );
    tl.play(Anim::fade_in_from(slider, Direction::Left, 120.0).during(0.6));

    // 5) save_state / restore：方块移到右侧后完整 morph 回快照
    let snap = tl.save_state(slider);
    tl.play(Anim::move_to(slider, (420.0, 415.0)).during(0.6));
    let _restored = tl.restore(slider, &snap);

    // 6) add_group + group：底部三个小方块步进淡入
    let dots: Vec<Item> = (0..3)
        .map(|i: i32| {
            Item::rect(Rect::new(0.0, 0.0, 30.0, 30.0))
                .with_fill(accent)
                .with_transform(Affine::translate((700.0 + f64::from(i) * 55.0, 420.0)))
        })
        .collect();
    let dot_ids = tl.add_group(dots);
    tl.group(
        dot_ids.iter().map(|&id| Anim::fade_in(id).during(0.4)),
        0.15,
    );
    tl
}
// ANCHOR_END

// ANCHOR: move_demo
/// M9.10 验收场景（P1-4）：MoveAlongPath 沿路径移动。
///
/// 标题淡入 → S 形轨迹曲线描边画出 → 小球沿轨迹弧长匀速移动，
/// 演示 `Anim::move_along_path` 的逐帧路径采样求值（匀速参数化 + 缓动）。
fn move_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    let font = Font::from_system(None::<&str>)?;
    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 1) 标题淡入
    let text_path = font.outline_text("MoveAlongPath", 64.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = tl.add(
        Item::from_path(text_path)
            .with_fill(fg)
            .with_transform(Affine::translate((dx, f64::from(h) * 0.12))),
    );
    tl.play(Anim::fade_in(title).during(0.7));

    // 2) S 形轨迹：三次贝塞尔（起点左下 → 控制点拉出 S 弯 → 终点右侧）
    let start = Point::new(160.0, 430.0);
    let mut track = BezPath::new();
    track.move_to(start);
    track.curve_to((320.0, 60.0), (640.0, 560.0), (800.0, 170.0));
    let track_id = tl.add(
        Item::from_path(track.clone())
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(track_id).during(1.2));

    // 3) 小球置于轨迹起点，沿轨迹弧长匀速移动
    let ball = tl.add(Item::circle(Circle::new(start, 14.0)).with_fill(fg));
    tl.wait(0.2);
    tl.play(Anim::move_along_path(ball, &track).during(2.4));

    tl.wait(0.7);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: shapes_demo
/// M9.11 验收场景（P1-5）：TransformMatchingShapes 任意片段结构匹配形变。
///
/// 标题淡入 → "MATH" 逐字形淡入 → `match_shapes` 几何签名配对 →
/// M/A/T 跨行形变（T 异位滑动）、H 淡出、S 淡入。
fn shapes_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::{Color, ItemId};

    let font = Font::from_system(None::<&str>)?;
    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 1) 标题淡入
    let text_path = font.outline_text("TransformMatchingShapes", 52.0).unwrap();
    let bounds = text_path.bounding_box();
    let dx = f64::from(w) / 2.0 - (bounds.x0 + bounds.x1) / 2.0;
    let title = tl.add(
        Item::from_path(text_path)
            .with_fill(fg)
            .with_transform(Affine::translate((dx, f64::from(h) * 0.12))),
    );
    tl.play(Anim::fade_in(title).during(0.7));

    // 逐字形散件：单字符 outline_text（同字形 base path 相同，位置烘焙在坐标里）
    let glyphs = |word: &str, size: f64, cx: f64, baseline: f64, color: Color| -> Vec<Item> {
        let adv = size * 0.78;
        let n = word.chars().count() as f64;
        word.chars()
            .enumerate()
            .map(|(i, c)| {
                let path = font.outline_text(&c.to_string(), size).unwrap();
                let x = cx - adv * (n - 1.0) / 2.0 + i as f64 * adv;
                Item::from_path(path)
                    .with_fill(color)
                    .with_transform(Affine::translate((x, baseline)))
            })
            .collect()
    };

    // 2) "MATH" 逐字形淡入
    let a_parts = glyphs("MATH", 96.0, 480.0, 250.0, accent);
    let ids_a: Vec<ItemId> = a_parts.iter().map(|it| tl.add(it.clone())).collect();
    tl.group(ids_a.iter().map(|&id| Anim::fade_in(id).during(0.8)), 0.08);
    tl.wait(0.4);

    // 3) match_shapes 配对 → M/A/T 形变（T 异位滑动）、H 淡出、S 淡入
    let b_parts = glyphs("MAST", 96.0, 480.0, 420.0, fg);
    let ids_b: Vec<ItemId> = b_parts.iter().map(|it| tl.add(it.clone())).collect();
    let plan = ffanim::match_shapes(&a_parts, &b_parts);
    let dur = 1.4;
    let anims = plan
        .paired
        .iter()
        .map(|&(i, j)| Anim::transform(ids_a[i], ids_b[j]).during(dur))
        .chain(
            plan.leave_a
                .iter()
                .map(|&k| Anim::fade_out(ids_a[k]).during(dur)),
        )
        .chain(
            plan.enter_b
                .iter()
                .map(|&k| Anim::fade_in(ids_b[k]).during(dur)),
        );
    tl.together(anims);
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END
