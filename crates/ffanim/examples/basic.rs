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
    "riemann_area",
    "line_graph",
    "angle_demo",
    "circle_sine_demo",
    "warp_grid_demo",
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
        "riemann_area" => riemann_area(theme, FPS),
        "line_graph" => line_graph(theme, FPS),
        "angle_demo" => angle_demo(theme, FPS),
        "circle_sine_demo" => circle_sine_demo(theme, FPS),
        "warp_grid_demo" => warp_grid_demo(theme, FPS),
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

// ANCHOR: riemann_area
/// GraphAreaPlot 对齐场景：曲线下面积 + 黎曼矩形近似（纯现有 API 拼装，∫₀^π sin x dx = 2）。
///
/// sin 曲线画出 → n=8 黎曼矩形逐块错峰入场 → 同位交叉淡切动态组（tracker 驱动）→
/// n 8→64 收敛 → 精确面积渐显叠加对比。
fn riemann_area(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Axes;

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let f = |x: f64| x.sin();
    let pi = std::f64::consts::PI;
    let axes = Axes::new((0.0, 10.0), (-1.3, 1.3))
        .with_size(840.0, 400.0)
        .with_center(f64::from(w) / 2.0, f64::from(h) / 2.0);

    // 网格 + 轴
    let grid = tl.add(axes.grid_item(fg.with_alpha(40), 1.0));
    tl.play(Anim::fade_in(grid).during(0.6));
    let ax = tl.add(axes.to_item(&theme.style));
    tl.overlap(0.4);
    tl.play(Anim::create(ax).during(1.0));

    // 函数曲线 [0, 10]
    let curve = tl.add(
        axes.plot(f, (0.0, 10.0))
            .with_stroke(accent)
            .with_stroke_width(3.0),
    );
    tl.wait(0.2);
    tl.play(Anim::create(curve).during(1.4));

    // 静态黎曼矩形 n=8：逐块错峰入场（manim lagged_start 同款）
    let rects = riemann_rects(
        &axes,
        f,
        8,
        0.0,
        pi,
        accent.with_alpha(90),
        accent.with_alpha(220),
    );
    let static_ids = tl.add_group(rects);
    tl.wait(0.2);
    tl.lagged_start(
        static_ids.iter().map(|&id| Anim::fade_in(id).during(0.4)),
        0.4,
    );

    // 动态黎曼矩形：n 由 tracker 驱动，updater 每帧重建整组；
    // 初始 n=8 与静态组同形，交叉淡化无缝切换 → n 8→64 收敛
    let n = tl.add_tracker(8.0);
    let fill = accent.with_alpha(90);
    let stroke = accent.with_alpha(220);
    let dyn_rects = tl.add_updater(&[n], move |v| {
        let n = (v[0].round() as usize).max(1);
        Item::group(riemann_rects(&axes, f, n, 0.0, pi, fill, stroke))
    });
    let mut cross: Vec<_> = static_ids
        .iter()
        .map(|&id| Anim::fade_out(id).during(0.5))
        .collect();
    cross.push(Anim::fade_in(dyn_rects).during(0.5));
    tl.together(cross);
    tl.play(Anim::set_value(n, 64.0).during(2.0));

    // 精确面积：曲线采样 + 底边折返闭合，半透明叠加对比矩形近似
    let area = tl.add(area_under(&axes, f, 0.0, pi, accent.with_alpha(60)));
    tl.wait(0.3);
    tl.play(Anim::fade_in(area).during(1.0));
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

/// 黎曼矩形组：中点法取样，4 顶点经 c2p 变换后闭合（正负段均成立）。
fn riemann_rects(
    axes: &ffanim::Axes,
    f: impl Fn(f64) -> f64,
    n: usize,
    x0: f64,
    x1: f64,
    fill: ffanim::Color,
    stroke: ffanim::Color,
) -> Vec<Item> {
    let dx = (x1 - x0) / n as f64;
    (0..n)
        .map(|i| {
            let xa = x0 + i as f64 * dx;
            let y = f(xa + dx / 2.0);
            let mut path = BezPath::new();
            path.move_to(axes.c2p(xa, 0.0));
            path.line_to(axes.c2p(xa + dx, 0.0));
            path.line_to(axes.c2p(xa + dx, y));
            path.line_to(axes.c2p(xa, y));
            path.close_path();
            Item::from_path(path)
                .with_fill(fill)
                .with_stroke(stroke)
                .with_stroke_width(1.0)
        })
        .collect()
}

/// 曲线下面积：沿曲线采样 a→b、底边 y=0 折返闭合；单值函数不自交，填充天然正确。
fn area_under(
    axes: &ffanim::Axes,
    f: impl Fn(f64) -> f64,
    a: f64,
    b: f64,
    fill: ffanim::Color,
) -> Item {
    const SAMPLES: usize = 120;
    let mut path = BezPath::new();
    path.move_to(axes.c2p(a, 0.0));
    for i in 0..=SAMPLES {
        let x = a + (b - a) * i as f64 / SAMPLES as f64;
        path.line_to(axes.c2p(x, f(x)));
    }
    path.line_to(axes.c2p(b, 0.0));
    path.close_path();
    Item::from_path(path).with_fill(fill)
}

// ANCHOR: line_graph
/// HeatDiagramPlot 对齐场景：折线/散点图（对标 Axes.plot_line_graph，纯现有 API 拼装）。
///
/// 12 点测量数据：折线 create → 顶点散点错峰入场 → 交叉淡切动态组 →
/// 相位 tracker 驱动曲线波动（数据实时更新语义）。
fn line_graph(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Axes;

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 示例数据：12 个月测量值（点列定义一次，静态/动态共用）
    const PTS: [(f64, f64); 12] = [
        (1.0, 2.1),
        (2.0, 3.4),
        (3.0, 2.8),
        (4.0, 4.6),
        (5.0, 5.2),
        (6.0, 4.1),
        (7.0, 5.8),
        (8.0, 6.5),
        (9.0, 5.4),
        (10.0, 7.2),
        (11.0, 6.1),
        (12.0, 7.8),
    ];
    let axes = Axes::new((0.0, 13.0), (0.0, 9.0))
        .with_size(840.0, 400.0)
        .with_center(f64::from(w) / 2.0, f64::from(h) / 2.0);

    // 网格 + 轴
    let grid = tl.add(axes.grid_item(fg.with_alpha(40), 1.0));
    tl.play(Anim::fade_in(grid).during(0.6));
    let ax = tl.add(axes.to_item(&theme.style));
    tl.overlap(0.4);
    tl.play(Anim::create(ax).during(1.0));

    // 折线描边生长
    let line = tl.add(polyline(&axes, &PTS, accent, 3.0));
    tl.wait(0.2);
    tl.play(Anim::create(line).during(1.2));

    // 顶点散点逐个错峰入场
    let dots = scatter(&axes, &PTS, 6.0, accent);
    let dot_ids = tl.add_group(dots);
    tl.lagged_start(dot_ids.iter().map(|&id| Anim::fade_in(id).during(0.3)), 0.4);

    // 动态组：相位 tracker 驱动 y 值正弦微扰，updater 每帧重建折线+散点
    let phase = tl.add_tracker(0.0);
    let stroke = accent;
    let dot_fill = accent;
    let dyn_graph = tl.add_updater(&[phase], move |v| {
        let pts: Vec<(f64, f64)> = PTS
            .iter()
            .map(|(x, y)| (*x, y + 0.8 * (x * 0.7 + v[0]).sin()))
            .collect();
        let mut children = vec![polyline(&axes, &pts, stroke, 3.0)];
        children.extend(scatter(&axes, &pts, 6.0, dot_fill));
        Item::group(children)
    });
    tl.wait(0.2);
    // 与静态组同形交叉淡化 → 相位 0→2π 波动一回合
    let mut cross: Vec<_> = dot_ids
        .iter()
        .map(|&id| Anim::fade_out(id).during(0.5))
        .collect();
    cross.push(Anim::fade_out(line).during(0.5));
    cross.push(Anim::fade_in(dyn_graph).during(0.5));
    tl.together(cross);
    tl.play(Anim::set_value(phase, std::f64::consts::TAU).during(2.0));

    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

/// 折线：数据点列 c2p 依次连线（plot_line_graph 的线部，open polyline）。
fn polyline(axes: &ffanim::Axes, pts: &[(f64, f64)], stroke: ffanim::Color, width: f64) -> Item {
    let mut path = BezPath::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        let p = axes.c2p(*x, *y);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    Item::from_path(path)
        .with_stroke(stroke)
        .with_stroke_width(width)
}

/// 散点：每点一个小实心圆（plot_line_graph 的点部）。
fn scatter(axes: &ffanim::Axes, pts: &[(f64, f64)], r: f64, fill: ffanim::Color) -> Vec<Item> {
    pts.iter()
        .map(|(x, y)| Item::circle(Circle::new(axes.c2p(*x, *y), r)).with_fill(fill))
        .collect()
}

// ANCHOR: angle_demo
/// MovingAngle 对齐场景：夹角测量件拼装（弧 + 双线 + θ 标注，纯现有 API）。
///
/// 固定线 + 动线（60°）+ 夹角弧 + θ 公式标注入场 → 交叉淡切动态组 →
/// 夹角 60°→15°→90° 摆动，弧与 θ 沿角平分线跟随（tracker 驱动逐帧重建）。
/// 几何常量：顶点偏下居中，上方留 90° 摆动空间；角度均为数学角（逆时针为正）。
const ANGLE_O: Point = Point::new(480.0, 380.0);
const ANGLE_LEN: f64 = 260.0;
const ANGLE_ARC_R: f64 = 70.0;

fn angle_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{FormulaOptions, TypstEngine};

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // θ 标注排版一次：中心锚定到原点的字形，动态段仅做仿射平移（无逐帧重排）
    let mut engine = TypstEngine::new(8);
    let theta_formula = engine.formula(
        "theta",
        &FormulaOptions {
            font_size_pt: 30.0,
            text_color: accent,
        },
    )?;
    let base = theta_formula.fit_transform(0.0, 0.0, 1.6);
    // 字形保留无 transform 的局部坐标，label 处合成 translate×base（with_transform 是替换语义）
    let theta_glyph = theta_formula.to_items().remove(0);

    // 静态段：顶点 → 固定线（0°）→ 动线（60°）→ 弧 + θ 标注
    let dot = tl.add(Item::circle(Circle::new(ANGLE_O, 5.0)).with_fill(fg));
    tl.play(Anim::fade_in(dot).during(0.3));
    let fixed = tl.add(
        Item::line(ANGLE_O, Point::new(ANGLE_O.x + ANGLE_LEN, ANGLE_O.y))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    tl.play(Anim::create(fixed).during(0.7));
    let parts = angle_group(&theta_glyph, base, 60.0, accent);
    let arm = tl.add(parts[0].clone());
    let arc_id = tl.add(parts[1].clone());
    let label_id = tl.add(parts[2].clone());
    tl.play(Anim::create(arm).during(0.7));
    tl.wait(0.2);
    tl.together([
        Anim::fade_in(arc_id).during(0.6),
        Anim::fade_in(label_id).during(0.6),
    ]);

    // 动态组：夹角 tracker 驱动（动线 + 弧 + θ 三件每帧重建，初始 60° 同形无缝切换）
    let angle = tl.add_tracker(60.0);
    let dyn_angle = tl.add_updater(&[angle], move |v| {
        Item::group(angle_group(&theta_glyph, base, v[0], accent))
    });
    tl.wait(0.3);
    let mut cross: Vec<_> = [arm, arc_id, label_id]
        .iter()
        .map(|&id| Anim::fade_out(id).during(0.5))
        .collect();
    cross.push(Anim::fade_in(dyn_angle).during(0.5));
    tl.together(cross);

    // 夹角摆动：收窄 → 张开到直角（Smooth 默认缓动）
    tl.play(Anim::set_value(angle, 15.0).during(1.2));
    tl.play(Anim::set_value(angle, 90.0).during(1.6));
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

/// 夹角三件套：动线（0°→theta_deg 数学角）+ 夹角弧 + θ 标注（角平分线方向）。
/// 动态/静态组共用同一构造，保证切换前后形状一致。
fn angle_group(
    theta_glyph: &Item,
    base: Affine,
    theta_deg: f64,
    stroke: ffanim::Color,
) -> Vec<Item> {
    let rad = theta_deg.to_radians();
    let tip = Point::new(
        ANGLE_O.x + ANGLE_LEN * rad.cos(),
        ANGLE_O.y - ANGLE_LEN * rad.sin(),
    );
    let arm = Item::line(ANGLE_O, tip)
        .with_stroke(stroke)
        .with_stroke_width(3.0);
    let arc = Item::arc(ANGLE_O, ANGLE_ARC_R, 0.0, theta_deg)
        .with_stroke(stroke)
        .with_stroke_width(2.0);
    // θ 标注：角平分线方向（数学角 θ/2），半径略大于弧；translate×base 复合锚定中心
    let mid = (theta_deg / 2.0).to_radians();
    let d = ANGLE_ARC_R + 45.0;
    let label = theta_glyph.clone().with_transform(
        Affine::translate((ANGLE_O.x + d * mid.cos(), ANGLE_O.y - d * mid.sin())) * base,
    );
    vec![arm, arc, label]
}

// ANCHOR: circle_sine_demo
/// SineCurveUnitCircle 对齐场景：单位圆动点绕转 ↔ 正弦曲线逐段展开（联动）。
///
/// 纯现有 API 拼装：圆 = 上下两半 plot（y=f(x) 单值约束的拆解）+ 同步 create
/// （两半同长反向生长 ≙ 单路径描边一圈）；正弦 = plot 直接；
/// 动点/曲线前沿点/引导线由 phase tracker 每帧重建 → 同窗同步：
/// 正弦曲线 create 与点绕圆一圈等长（同 Smooth rate，进度一致）。
fn circle_sine_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Axes;

    let accent = theme.accent().unwrap();
    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let tau = std::f64::consts::TAU;
    // 左：单位圆区（数据域 [-1.4,1.4]²）；右：正弦区（x 数据域 [0, 2π] 与相位同标度）
    let circle_axes = Axes::new((-1.4, 1.4), (-1.4, 1.4))
        .with_size(260.0, 260.0)
        .with_center(190.0, 270.0);
    let sine_axes = Axes::new((0.0, tau), (-1.4, 1.4))
        .with_size(500.0, 260.0)
        .with_center(650.0, 270.0);

    // 轴与网格
    let grid1 = tl.add(circle_axes.grid_item(fg.with_alpha(40), 1.0));
    let grid2 = tl.add(sine_axes.grid_item(fg.with_alpha(40), 1.0));
    tl.together([
        Anim::fade_in(grid1).during(0.6),
        Anim::fade_in(grid2).during(0.6),
    ]);
    let ax1 = tl.add(circle_axes.to_item(&theme.style));
    let ax2 = tl.add(sine_axes.to_item(&theme.style));
    tl.overlap(0.3);
    tl.together([Anim::create(ax1).during(1.0), Anim::create(ax2).during(1.0)]);

    // 单位圆：拆上下两半各 plot（圆非 y=f(x) 单值，两半拼接 ≙ 完整圆）
    let circle_u = tl.add(
        circle_axes
            .plot(|x| (1.0 - x * x).sqrt(), (-1.0, 1.0))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    let circle_l = tl.add(
        circle_axes
            .plot(|x| -(1.0 - x * x).sqrt(), (-1.0, 1.0))
            .with_stroke(fg)
            .with_stroke_width(3.0),
    );
    // 两半同步生长：都从左端点 (-1,0) 长到右端点 (1,0)，合成"从左侧双向展开一圈"
    tl.together([
        Anim::create(circle_u).during(1.2),
        Anim::create(circle_l).during(1.2),
    ]);

    // 正弦曲线：y=sin(x) 单值，plot 直接；x 数据域 [0, 2π] 恰好铺满
    let sine = tl.add(
        sine_axes
            .plot(|x| x.sin(), (0.0, tau))
            .with_stroke(accent)
            .with_stroke_width(3.0),
    );

    // 联动三件套：圆上动点 + 曲线对应点 + 跨区引导线（phase 每帧重建，全程可见）
    let phase = tl.add_tracker(0.0);
    tl.add_updater(&[phase], move |v| {
        let t = v[0];
        let c_pt = circle_axes.c2p(t.cos(), t.sin());
        let s_pt = sine_axes.c2p(t, t.sin());
        let conn = Item::line(c_pt, s_pt)
            .with_stroke(fg.with_alpha(120))
            .with_stroke_width(1.5);
        let dot_c = Item::circle(Circle::new(c_pt, 8.0)).with_fill(accent);
        let dot_s = Item::circle(Circle::new(s_pt, 8.0)).with_fill(accent);
        Item::group(vec![conn, dot_c, dot_s])
    });

    // 同窗同步：曲线逐段生长 ↔ 点绕圆一圈（同 duration 同 rate，进度一致）
    tl.wait(0.2);
    tl.together([
        Anim::create(sine).during(1.8),
        Anim::set_value(phase, tau).during(1.8),
    ]);
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END

// ANCHOR: warp_grid_demo
/// OpeningManim 对齐场景：网格逐点非线性形变（`apply_function` 对标）。
///
/// 纯现有 API 拼装：网格 = 自采样折线构造（`c2p`/`from_path` 公开）；
/// 形变 = 逐点 `p → lerp(p, f(p), t)` 作用于数据坐标；tracker 强度
/// 0→1→0 驱动 updater 每帧重建，形变与复原同构（纯函数语义）。
fn warp_grid_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::Axes;

    let fg = theme.foreground().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let axes = Axes::new((-3.0, 3.0), (-2.0, 2.0))
        .with_size(780.0, 360.0)
        .with_center(480.0, 270.0);
    // 形变映射：数据域波浪扭曲（apply_function 的逐点 λp）
    let warp = |x: f64, y: f64| (x + 0.25 * (2.0 * y).sin(), y + 0.25 * (2.0 * x).sin());

    // 强度 tracker：0 = 直网格，1 = 完全形变；updater 每帧重建整张网格
    let strength = tl.add_tracker(0.0);
    let grid = tl.add_updater(&[strength], move |v| {
        warp_grid_item(&axes, warp, v[0], fg.with_alpha(90))
    });
    let ax = tl.add(axes.to_item(&theme.style));
    tl.together([
        Anim::fade_in(grid).during(0.8),
        Anim::create(ax).during(0.8),
    ]);

    tl.wait(0.3);
    tl.play(Anim::set_value(strength, 1.0).during(1.6)); // 形变
    tl.wait(0.5);
    tl.play(Anim::set_value(strength, 0.0).during(1.4)); // 复原
    tl.wait(0.8);
    Ok(tl)
}

/// 逐点形变网格：竖横两组线各离散为折线，采样点经 `p → lerp(p, f(p), t)`
/// 后 `c2p`。t=0 时 lerp 恒等 ≙ 直网格——这是 apply_function 的拼装实现。
fn warp_grid_item(
    axes: &ffanim::Axes,
    f: impl Fn(f64, f64) -> (f64, f64),
    t: f64,
    color: ffanim::Color,
) -> Item {
    const K: usize = 32; // 每条线离散段数
    let (x0, x1) = axes.x_range;
    let (y0, y1) = axes.y_range;
    let mut path = BezPath::new();
    // 竖线族：x = 整数步进，沿线 y 离散采样
    let mut x = x0.ceil();
    while x <= x1 {
        for i in 0..=K {
            let y = y0 + (y1 - y0) * i as f64 / K as f64;
            let (wx, wy) = f(x, y);
            let p = axes.c2p(x + (wx - x) * t, y + (wy - y) * t);
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        x += 1.0;
    }
    // 横线族：y = 整数步进，沿线 x 离散采样
    let mut y = y0.ceil();
    while y <= y1 {
        for i in 0..=K {
            let x = x0 + (x1 - x0) * i as f64 / K as f64;
            let (wx, wy) = f(x, y);
            let p = axes.c2p(x + (wx - x) * t, y + (wy - y) * t);
            if i == 0 {
                path.move_to(p);
            } else {
                path.line_to(p);
            }
        }
        y += 1.0;
    }
    Item::from_path(path)
        .with_stroke(color)
        .with_stroke_width(1.0)
}
// ANCHOR_END
