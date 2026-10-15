//! 文字与公式场景族：公式 Write/TransformMatchingTex / 富文本 / 标注复合件 / 代码高亮 / t2c 按词着色。
//!
//! 对齐 manim `example_scenes/` 形态：单文件多场景 + argv 寻址。
//!
//! 运行（仓库根目录）：
//! `cargo run -p ffanim --release --example typography -- [场景名...]`（缺省全渲染，≙ `manim -a`）
//! `cargo run -p ffanim --example typography -- --json text_demo`（打印调度表，agent 检查用）

use ffanim::kurbo::Affine;
use ffanim::{Anim, Item, Theme, Timeline};

const FPS: u32 = 30;

/// 场景清单（argv 寻址，缺省全渲染）。
const SCENES: &[&str] = &[
    "formula_demo",
    "text_demo",
    "annotate_demo",
    "code_demo",
    "t2c_demo",
];

fn build(name: &str, theme: &Theme) -> ffanim::Result<Timeline> {
    match name {
        "formula_demo" => formula_demo(theme, FPS),
        "text_demo" => text_demo(theme, FPS),
        "annotate_demo" => annotate_demo(theme, FPS),
        "code_demo" => code_demo(theme, FPS),
        "t2c_demo" => t2c_demo(theme, FPS),
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

// ANCHOR: formula_demo
/// M3 验收场景：公式逐字形写出（Write）→ 结构匹配形变（TransformMatchingTex）。
///
/// `x^2 + y^2 = z^2` 逐字形写出后，`+`/`z^2` 淡出、`-`/`0` 淡入，
/// 相同子式（x、2、y、=）跨公式逐点形变。
fn formula_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::FormulaOptions;
    use ffanim::{ItemId, RateFunc};

    let mut engine = ffanim::typst::TypstEngine::new(16);
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
    let place = |f: &ffanim::typst::Formula| f.fit_transform(w / 2.0, h / 2.0, scale);

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
    let plan = ffanim::typst::match_formulas(&f1, &f2);
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
// ANCHOR_END

// ANCHOR: text_demo
/// M8.2 验收场景：Text 富文本——中英混排限宽不出框 + 粗细混排 + 行距。
///
/// 系统中文字体经 `find_cjk_font` 数据查找注入引擎（无硬编码路径），
/// 文本走 Typst markup 管线（与公式共用 Formula 产物）。
fn text_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 引擎 + 系统中文字体（env FFANIM_CJK_FONT → fc-list → 常见路径）
    let mut engine = TypstEngine::new(16);
    if let Some((font, idx)) = find_cjk_font() {
        engine.add_font_face(&font, idx)?;
    }

    // 排版产物落位 + 整组同时淡入（group 步长 0 = 同窗）
    let show = |tl: &mut Timeline, f: &ffanim::typst::Formula, cy: f64, dur: f64| {
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
// ANCHOR_END

// ANCHOR: annotate_demo
/// manim 对齐 P1-7 验收场景：标注复合件（圈选框/下划线/叉/背景板）。
///
/// 四个标注几何构造器均基于目标 bbox（含 transform）计算；标注基于注册基线，
/// 目标有位姿动画时需与标注同步挂相同位姿（第 1 段演示）。
fn annotate_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{FormulaOptions, TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let mut engine = TypstEngine::new(16);
    if let Some((font, idx)) = find_cjk_font() {
        engine.add_font_face(&font, idx)?;
    }
    let fopts = FormulaOptions {
        font_size_pt: 26.0,
        text_color: fg,
    };
    let topt = |size: f64, color: ffanim::Color| TextOptions::new(size, color);

    // 排版产物注册 helper：散件注册（返回句柄，供字形级动画）+ 拼组（仅几何用途，不注册）
    let reg = |tl: &mut Timeline, f: &ffanim::typst::Formula, place: Affine| -> (Vec<_>, Item) {
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
    let bg =
        tl.add(Item::background_rect(&cap_grp, 16.0).with_fill(ffanim::Color { a: 90, ..accent }));
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
// ANCHOR_END

// ANCHOR: code_demo
/// M9.9 验收场景（P1-13）：代码块语法高亮。
///
/// 标题淡入 → Rust 代码块逐字形写出（Dark 主题）→ Python 代码块淡入，
/// 演示 `engine.code` + 内嵌深色 tmTheme 的多色高亮。
fn code_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{CodeOptions, TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    let sw = f64::from(theme.style.stroke_width);
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    let mut engine = TypstEngine::new(16);
    if let Some((font, idx)) = find_cjk_font() {
        engine.add_font_face(&font, idx)?;
    }

    // 排版产物注册 helper：散件注册（供逐字形动画）+ 拼组（仅几何用途）
    let reg = |tl: &mut Timeline, f: &ffanim::typst::Formula, place: Affine| -> (Vec<_>, Item) {
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
// ANCHOR_END

// ANCHOR: t2c_demo
/// M9.16 验收场景：P1-12 按词着色（manim `Text(t2c=…)` 对应）。
///
/// 标题淡入 → 中文句重复词全出现着色（数学/动画各两处）→ 英文句
/// 混合色 span（Palette 三色派生第二高亮：lerp(前景, 强调, 0.55)）
/// → 结论行。着色文本 = Typst markup 注入 `#text(fill:)[词]`，
/// 出场 fade_in（闭合填充字形禁 create）。
fn t2c_demo(theme: &Theme, fps: u32) -> ffanim::Result<Timeline> {
    use ffanim::typst::{TextOptions, TypstEngine, find_cjk_font};

    let fg = theme.foreground().unwrap();
    let accent = theme.accent().unwrap();
    // Palette 仅三色：第二高亮色由主题色派生（数据驱动，不硬编码色值）
    let mid = ffanim::Color::lerp(fg, accent, 0.55);
    let (w, h) = (960u32, 540u32);
    let mut tl = Timeline::new(w, h, fps, theme.background().unwrap());

    // 中文正文须 CJK 字形（缺失时回退拉丁字体，中文会口占位但仍出片）
    let mut engine = TypstEngine::new(16);
    if let Some((font, idx)) = find_cjk_font() {
        engine.add_font_face(&font, idx)?;
    }

    let show = |tl: &mut Timeline, f: &ffanim::typst::Formula, cy: f64, dur: f64| {
        let place = f.fit_transform(f64::from(w) / 2.0, cy, 2.0); // pt → px
        let ids: Vec<_> = f
            .to_items()
            .into_iter()
            .map(|item| tl.add(item.with_transform(place)))
            .collect();
        tl.group(ids.iter().map(|&id| Anim::fade_in(id).during(dur)), 0.0);
    };

    // 标题
    let title = engine.text("ffanim *按词着色* t2c", &TextOptions::new(28.0, fg))?;
    show(&mut tl, &title, 100.0, 0.8);
    tl.wait(0.4);

    // 中文句：重复词全出现着色（数学 ×2、动画 ×2）
    let cn = engine.text_t2c(
        "数学驱动动画，动画承载数学。",
        &[("数学", accent), ("动画", mid)],
        &TextOptions::new(24.0, fg),
    )?;
    show(&mut tl, &cn, 250.0, 0.8);
    tl.wait(0.4);

    // 英文句：特殊字符（括号/等号）自动转义 + 双 span
    let en = engine.text_t2c(
        "Text(t2c = every match gets its color)",
        &[("t2c", accent), ("color", mid)],
        &TextOptions::new(22.0, fg),
    )?;
    show(&mut tl, &en, 370.0, 0.8);
    tl.wait(0.4);

    // 结论行
    let cap = engine.text(
        "全出现 · 对序优先 · 非重叠 · 主题色驱动",
        &TextOptions::new(18.0, accent),
    )?;
    show(&mut tl, &cap, 480.0, 0.8);
    tl.wait(0.8);
    Ok(tl)
}
// ANCHOR_END
