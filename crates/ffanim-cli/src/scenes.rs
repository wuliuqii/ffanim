//! 内置示例场景注册表。
//!
//! M2 验收场景 `demo_anim`：Create / FadeIn / Transform 三种动画词汇串演。

use crate::demo::load_font;
use ffanim::Theme;
use ffanim_core::kurbo::Shape as _;
use ffanim_core::kurbo::{Affine, BezPath, Circle, Point, Rect};
use ffanim_core::{Anim, Font, Item, Timeline};

/// 场景清单（M5 起支持外部场景文件）。
pub const SCENES: &[&str] = &[
    "demo_anim",
    "formula_demo",
    "sine_demo",
    "text_demo",
    "primitives_demo",
    "layout_demo",
    "emphasize_demo",
    "sugar_demo",
    "annotate_demo",
    "code_demo",
    "move_demo",
    "shapes_demo",
    "brace_demo",
    "delimiter_demo",
    "arrow_demo",
];

/// 构建场景时间轴。
pub fn build(name: &str, theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    match name {
        "demo_anim" => {
            let font = load_font(None)?;
            Ok(demo_anim(theme, &font, fps))
        }
        "formula_demo" => formula_demo(theme, fps),
        "sine_demo" => sine_demo(theme, fps),
        "text_demo" => text_demo(theme, fps),
        "primitives_demo" => Ok(primitives_demo(theme, fps)),
        "layout_demo" => Ok(layout_demo(theme, fps)),
        "emphasize_demo" => Ok(emphasize_demo(theme, fps)),
        "sugar_demo" => Ok(sugar_demo(theme, fps)),
        "annotate_demo" => annotate_demo(theme, fps),
        "code_demo" => code_demo(theme, fps),
        "move_demo" => move_demo(theme, fps),
        "shapes_demo" => shapes_demo(theme, fps),
        "brace_demo" => brace_demo(theme, fps),
        "delimiter_demo" => delimiter_demo(theme, fps),
        "arrow_demo" => arrow_demo(theme, fps),
        other => Err(ffanim::Error::Other(format!(
            "未知场景 {other}，可用场景: {}",
            SCENES.join(", ")
        ))),
    }
}

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
    let cir_center: ffanim_core::kurbo::Point = (f64::from(w) * 0.75, f64::from(h) * 0.62).into();
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

/// M3 验收场景：公式逐字形写出（Write）→ 结构匹配形变（TransformMatchingTex）。
///
/// `x^2 + y^2 = z^2` 逐字形写出后，`+`/`z^2` 淡出、`-`/`0` 淡入，
/// 相同子式（x、2、y、=）跨公式逐点形变。
fn formula_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_core::{ItemId, RateFunc};
    use ffanim_typst::FormulaOptions;

    let mut engine = ffanim_typst::TypstEngine::new(16);
    let fopts = FormulaOptions {
        font_size_pt: 32.0,
        text_color: theme.foreground().unwrap(),
    };
    let f1 = engine.formula("x^2 + y^2 = z^2", &fopts)?;
    let f2 = engine.formula("x^2 - y^2 = 0", &fopts)?;

    let (w, h) = (960.0_f64, 540.0_f64);
    let scale = 2.0; // pt → px
    let mut tl = Timeline::new(w as u32, h as u32, fps, theme.background().unwrap());
    // 公式局部坐标 → 画布居中（fit_transform：缩放后的中心精确落在目标点）
    let place = |f: &ffanim_typst::Formula| f.fit_transform(w / 2.0, h / 2.0, scale);

    // —— Write：逐字形 Create，group 按步长错开排入（M8.3 组动画，替代 overlap 手工回退）——
    let write_dur = 0.5;
    let step = 0.06;
    let ids_a: Vec<ItemId> = f1
        .to_items()
        .into_iter()
        .map(|item| tl.add(item.with_transform(place(&f1))))
        .collect();
    tl.group(
        ids_a
            .iter()
            .map(|&id| Anim::create(id).during(write_dur).rate(RateFunc::Smooth)),
        step,
    );
    tl.wait(0.6);

    // —— TransformMatchingTex：配对形变，其余淡出/淡入（同一窗口并行）——
    let plan = ffanim_typst::match_formulas(&f1, &f2);
    let mut ids_b: Vec<ItemId> = Vec::new();
    for item in f2.to_items() {
        ids_b.push(tl.add(item.with_transform(place(&f2))));
    }

    let dur = 1.2;
    let mut anims: Vec<Anim> = Vec::new();
    for (i, j) in &plan.paired {
        anims.push(Anim::transform(ids_a[*i], ids_b[*j]).during(dur));
    }
    // 淡出/淡入与形变同时长，保证所有动画共享同一窗口（overlap 回退对齐）
    for &k in &plan.leave_a {
        anims.push(Anim::fade_out(ids_a[k]).during(dur));
    }
    for &k in &plan.enter_b {
        anims.push(Anim::fade_in(ids_b[k]).during(dur));
    }
    for (k, anim) in anims.into_iter().enumerate() {
        if k > 0 {
            // 回退自身 during → 与首个动画同窗口起点（并行形变）
            tl.overlap(anim.duration_s);
        }
        tl.play(anim);
    }
    tl.wait(0.8);
    Ok(tl)
}

/// M4 验收场景：坐标系 + 正弦函数图像随 ValueTracker 动画（求值期信号）。
///
/// 网格淡入 → 坐标轴描边生长 → sin 曲线逐段画出 → 振幅 1.0→0.2→1.0 呼吸。
fn sine_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_core::Axes;

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

/// M8.2 验收场景：Text 富文本——中英混排限宽不出框 + 粗细混排 + 行距。
///
/// 系统中文字体经 `find_cjk_font` 数据查找注入引擎（无硬编码路径），
/// 文本走 Typst markup 管线（与公式共用 Formula 产物）。
fn text_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_typst::{TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 引擎 + 系统中文字体（env FFANIM_CJK_FONT → fc-list → 常见路径）
    let mut engine = TypstEngine::new(16);
    if let Some(font) = find_cjk_font() {
        engine.add_font(&font)?;
    }

    // 排版产物落位 + 整组同时淡入（group 步长 0 = 同窗）
    let show = |tl: &mut Timeline, f: &ffanim_typst::Formula, cy: f64, dur: f64| {
        let place = f.fit_transform(f64::from(w) / 2.0, cy, 2.0); // pt → px
        let ids: Vec<_> = f
            .to_items()
            .into_iter()
            .map(|item| tl.add(item.with_transform(place)))
            .collect();
        tl.group(ids.iter().map(|&id| Anim::fade_in(id).during(dur)), 0.0);
    };

    // 标题：中英混排 + `*…*` 粗体混排（拉丁粗体来自 typst-assets 内置 Bold）
    let title = engine.text("ffanim *富文本* Text 排版", &TextOptions::new(28.0, fg))?;
    show(&mut tl, &title, 100.0, 0.8);
    tl.wait(0.4);

    // 段落：限宽 220pt（缩放后 440px）→ 中英混排自动换行不出框，行距 1.1
    let para = engine.text(
        "数学动画引擎支持中英混排 mixed Chinese-English text，限宽后自动换行 \
         line wrapping，行距可调 adjustable leading。",
        &TextOptions {
            wrap_width_pt: Some(220.0),
            leading_em: 1.1,
            ..TextOptions::new(15.0, fg)
        },
    )?;
    show(&mut tl, &para, 280.0, 0.8);
    tl.wait(0.6);

    // 结论行：强调色
    let cap = engine.text(
        "字号 · 行距 · 限宽 · 粗细 全部选项驱动",
        &TextOptions::new(18.0, accent),
    )?;
    show(&mut tl, &cap, 460.0, 0.8);
    tl.wait(0.8);
    Ok(tl)
}

/// manim 对齐 P0 验收场景：标注基元（线/箭头/虚线/弧/椭圆）+ 位姿动作糖。
///
/// 前半段基元逐个描边生长（group 错开），后半段演示 `shift_to / rotate_to /
/// scale_to / move_to` 四个相对位姿动作（构建期解析为绝对 TransformTo）。
fn primitives_demo(theme: &Theme, fps: u32) -> Timeline {
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
    let arrow = tl.add(
        Item::arrow((380.0, 120.0), (560.0, 120.0))
            .with_stroke(accent)
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
    let ids = [line, arrow, dash, arc, ell];
    tl.group(ids.iter().map(|&id| Anim::create(id).during(0.8)), 0.15);
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

/// manim 对齐 P0-2 验收场景：相对布局 next_to / to_edge / arrange。
///
/// 标题块贴顶边（to_edge）→ 三基元横排居中（arrange + 总包围盒平移）→
/// 步骤列表竖排（arrange）；全部为构建期布局，动画只做淡入/描边生长。
fn layout_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim_core::kurbo::Vec2;
    use ffanim_core::{Direction, ItemId, arrange};

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

/// M9.5 验收场景：强调四件套（indicate / circumscribe / flash / show_passing_flash）。
///
/// 对应 manim 的 Indicate / Circumscribe / Flash / ShowPassingFlash；
/// 全部为构建期编排（两段/三段既有词汇），求值路径零改动。
fn emphasize_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim_core::kurbo::BezPath;

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

/// M9.7 验收场景：P1 薄糖批（Uncreate / Together / Succession / 方向家族 /
/// add_group / save_state-restore）。
fn sugar_demo(theme: &Theme, fps: u32) -> Timeline {
    use ffanim_core::Direction;

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

    // 3) Succession：三支箭头依次描边生长
    let arrows: Vec<Item> = (0..3)
        .map(|i: i32| {
            let x = f64::from(i) * 200.0;
            Item::arrow((150.0 + x, 280.0), (260.0 + x, 280.0))
                .with_stroke(fg)
                .with_fill(fg)
                .with_stroke_width(sw)
        })
        .collect();
    let arrow_ids = tl.add_group(arrows);
    tl.succession([
        Anim::create(arrow_ids[0]).during(0.4),
        Anim::create(arrow_ids[1]).during(0.4),
        Anim::create(arrow_ids[2]).during(0.4),
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

/// manim 对齐 P1-7 验收场景：标注复合件（圈选框/下划线/叉/背景板）。
///
/// 四个标注几何构造器均基于目标 bbox（含 transform）计算；标注基于注册基线，
/// 目标有位姿动画时需与标注同步挂相同位姿（第 1 段演示）。
fn annotate_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_typst::{FormulaOptions, TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let mut engine = TypstEngine::new(16);
    if let Some(font) = find_cjk_font() {
        engine.add_font(&font)?;
    }
    let fopts = FormulaOptions {
        font_size_pt: 26.0,
        text_color: fg,
    };
    let topt = |size: f64, color: ffanim_core::Color| TextOptions::new(size, color);

    // 排版产物注册 helper：散件注册（返回句柄，供字形级动画）+ 拼组（仅几何用途，不注册）
    let reg = |tl: &mut Timeline, f: &ffanim_typst::Formula, place: Affine| -> (Vec<_>, Item) {
        let pieces: Vec<Item> = f
            .to_items()
            .into_iter()
            .map(|it| it.with_transform(place))
            .collect();
        let grp = Item::group(pieces.clone());
        (tl.add_group(pieces), grp)
    };

    // 1) 公式圈选：公式淡入 → surrounding_rect 描边生长 → 公式+框同步右移
    //    （标注基于注册基线，目标平移则标注挂相同 shift_to）
    let f = engine.formula("e^{i pi} + 1 = 0", &fopts)?;
    let (f_ids, f_grp) = reg(&mut tl, &f, f.fit_transform(f64::from(w) / 2.0, 120.0, 2.0));
    tl.group(f_ids.iter().map(|&id| Anim::fade_in(id).during(0.8)), 0.0);
    tl.wait(0.3);
    let frame = tl.add(
        Item::surrounding_rect(&f_grp, 18.0)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(frame).during(0.7));
    tl.wait(0.3);
    let mut sync = vec![Anim::shift_to(frame, 180.0, 0.0).during(0.8)];
    sync.extend(
        f_ids
            .iter()
            .map(|&id| Anim::shift_to(id, 180.0, 0.0).during(0.8)),
    );
    tl.together(sync);

    // 2) 下划线：单词淡入 → 底线描边
    let word = engine.text("underlined", &topt(24.0, fg))?;
    let (w_ids, w_grp) = reg(&mut tl, &word, word.fit_transform(240.0, 300.0, 2.0));
    tl.group(w_ids.iter().map(|&id| Anim::fade_in(id).during(0.6)), 0.0);
    tl.wait(0.2);
    let ul = tl.add(
        Item::underline(&w_grp, 10.0)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(ul).during(0.5));
    tl.wait(0.2);

    // 3) 叉：错误式子淡入 → 对角双线划掉
    let wrong = engine.text("2 + 2 = 5", &topt(24.0, fg))?;
    let (c_ids, c_grp) = reg(&mut tl, &wrong, wrong.fit_transform(700.0, 300.0, 2.0));
    tl.group(c_ids.iter().map(|&id| Anim::fade_in(id).during(0.6)), 0.0);
    tl.wait(0.3);
    let cross = tl.add(
        Item::cross(&c_grp, 12.0)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(cross).during(0.5));
    tl.wait(0.2);

    // 4) 背景板：先注册背景矩形（画在下）再注册文字（画在上），同窗淡入
    let cap = engine.text("背景板", &topt(28.0, fg))?;
    let cap_place = cap.fit_transform(f64::from(w) / 2.0, 450.0, 2.0);
    let cap_grp = Item::group(
        cap.to_items()
            .into_iter()
            .map(|it| it.with_transform(cap_place))
            .collect(),
    );
    let bg = tl.add(
        Item::background_rect(&cap_grp, 16.0).with_fill(ffanim_core::Color { a: 90, ..accent }),
    );
    let cap_ids = tl.add_group(
        cap.to_items()
            .into_iter()
            .map(|it| it.with_transform(cap_place))
            .collect(),
    );
    tl.together(
        [Anim::fade_in(bg).during(0.7)]
            .into_iter()
            .chain(cap_ids.iter().map(|&id| Anim::fade_in(id).during(0.7))),
    );
    tl.wait(0.5);
    Ok(tl)
}

/// M9.9 验收场景（P1-13）：代码块语法高亮。
///
/// 标题淡入 → Rust 代码块逐字形写出（Dark 主题）→ Python 代码块淡入，
/// 演示 `engine.code` + 内嵌深色 tmTheme 的多色高亮。
fn code_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_typst::{CodeOptions, TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let mut engine = TypstEngine::new(16);
    if let Some(font) = find_cjk_font() {
        engine.add_font(&font)?;
    }

    // 排版产物注册 helper：散件注册（供逐字形动画）+ 拼组（仅几何用途）
    let reg = |tl: &mut Timeline, f: &ffanim_typst::Formula, place: Affine| -> (Vec<_>, Item) {
        let pieces: Vec<Item> = f
            .to_items()
            .into_iter()
            .map(|it| it.with_transform(place))
            .collect();
        let grp = Item::group(pieces.clone());
        (tl.add_group(pieces), grp)
    };

    // 1) 标题淡入
    let title = engine.text("代码块高亮", &TextOptions::new(32.0, fg))?;
    let (t_ids, _) = reg(
        &mut tl,
        &title,
        title.fit_transform(f64::from(w) / 2.0, 70.0, 2.0),
    );
    tl.group(t_ids.iter().map(|&id| Anim::fade_in(id).during(0.8)), 0.0);

    // 2) Rust 代码块：逐字形写出（深色主题，关键字/字符串/数字/类型多色）
    let rust_code = "fn fib(n: u64) -> u64 {\n    match n {\n        0 | 1 => n,\n        _ => fib(n - 1) + fib(n - 2),\n    }\n}";
    let opts = CodeOptions::new(13.0);
    let code = engine.code(rust_code, "rust", &opts)?;
    let (c_ids, c_grp) = reg(&mut tl, &code, code.fit_transform(300.0, 280.0, 2.0));
    tl.group(c_ids.iter().map(|&id| Anim::create(id).during(1.6)), 0.0);
    tl.wait(0.3);

    // 3) 代码框标注（surrounding_rect）圈出代码块
    let frame = tl.add(
        Item::surrounding_rect(&c_grp, 20.0)
            .with_stroke(accent)
            .with_stroke_width(sw),
    );
    tl.play(Anim::create(frame).during(0.6));

    // 4) Python 代码块：右下方淡入（同为深色主题，多语言高亮演示）
    let py_code = "def greet(name: str) -> str:\n    return f\"hello, {name}\"";
    let py = engine.code(py_code, "python", &opts)?;
    let (p_ids, _) = reg(&mut tl, &py, py.fit_transform(620.0, 445.0, 2.0));
    tl.group(p_ids.iter().map(|&id| Anim::fade_in(id).during(0.9)), 0.0);
    tl.wait(0.6);
    Ok(tl)
}

/// M9.10 验收场景（P1-4）：MoveAlongPath 沿路径移动。
///
/// 标题淡入 → S 形轨迹曲线描边画出 → 小球沿轨迹弧长匀速移动，
/// 演示 `Anim::move_along_path` 的逐帧路径采样求值（匀速参数化 + 缓动）。
fn move_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    let font = load_font(None)?;
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

/// M9.11 验收场景（P1-5）：TransformMatchingShapes 任意片段结构匹配形变。
///
/// 标题淡入 → "MATH" 逐字形淡入 → `match_shapes` 几何签名配对 →
/// M/A/T 跨行形变（T 异位滑动）、H 淡出、S 淡入。
fn shapes_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_core::{Color, ItemId};

    let font = load_font(None)?;
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

/// M9.12 验收场景（P1-8）：Brace 花括号 + 括注文本。
///
/// 标题淡入 → 横条描边生长 → 下方花括号淡入放大 + 括注“宽度”淡入 →
/// 右侧花括号 + 括注“高度”（manim `Brace` / `brace.get_text` 对应）。
/// 花括号为填充字形，出场用 fade_in（manim `GrowFromCenter` 对应）而非 create。
fn brace_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_core::Direction;
    use ffanim_typst::TypstEngine;

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim_typst::find_cjk_font()
        .and_then(|path| ffanim::Font::from_path(&path).ok())
        .map(Ok)
        .unwrap_or_else(|| load_font(None))?;
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

/// M9.13 验收场景：Delimiter 四类定界符统一 Typst MATH 装配。
///
/// 标题淡入 → 横条生长 → 花括号↓ / 方括号↑ / 圆括号← / 双线方括号→
/// 依次淡入贴四侧 + 中文括注（manim `Brace`/`Bracket`/`Parenthesis`/
/// `DoubleBracket` 对应；四类共用同一装配核心，仅符号与深度比不同）。
fn delimiter_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_core::Direction;
    use ffanim_typst::{Delimiter, TypstEngine};

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim_typst::find_cjk_font()
        .and_then(|path| ffanim::Font::from_path(&path).ok())
        .map(Ok)
        .unwrap_or_else(|| load_font(None))?;
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

/// arrow_demo：Typst 装配箭头验收示例（M9.14）。
///
/// 标题淡入 → 三支箭头（水平单头 / 斜向单头 / 水平双头）依次淡入 +
/// 中文括注（manim `Arrow`/`DoubleArrow` 对应；头高随跨度自适应并封顶，
/// 双头两端对称）。形状 = Typst MATH stretch 装配（NewCMMath GlyphAssembly），
/// 闭合填充字形，出场用 fade_in（填充字形禁用 create）。
fn arrow_demo(theme: &Theme, fps: u32) -> ffanim_core::Result<Timeline> {
    use ffanim_typst::{ArrowKind, TypstEngine};

    // 括注为中文：须 CJK 字形（find_cjk_font 优先，缺失时回退系统拉丁字体）
    let font = ffanim_typst::find_cjk_font()
        .and_then(|path| ffanim::Font::from_path(&path).ok())
        .map(Ok)
        .unwrap_or_else(|| load_font(None))?;
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
