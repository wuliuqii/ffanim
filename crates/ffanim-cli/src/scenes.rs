//! 内置示例场景注册表。
//!
//! M2 验收场景 `demo_anim`：Create / FadeIn / Transform 三种动画词汇串演。

use crate::demo::load_font;
use ffanim::Theme;
use ffanim_core::kurbo::Shape as _;
use ffanim_core::kurbo::{Affine, Circle, Rect};
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
