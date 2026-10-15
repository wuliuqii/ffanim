//! ffanim 创作示例：傅里叶级数逼近方波（M6 实际创作验证）。
//!
//! 真实用户视角走通全流程：主题 JSON → 公式 Write → 坐标系 + 方波目标 →
//! 部分和 S_N 随 ValueTracker 逐阶升高（吉布斯过冲可见）→ 渲染出片 →
//! strip/onion 诊断（agent 无看片验证循环的库式用法）。
//!
//! 运行（仓库根目录）：
//! `cargo run -p ffanim --release --example fourier`（需 PATH 有 ffmpeg）
//! 产物：out/fourier.mp4 + strip/onion 诊断 PNG。
//! 配音：`BGM=路径.wav cargo run ...` 自动混流并延伸片长（FromAudio 语义）。
//! 示例 BGM：Erik Satie《Gymnopédie No.1》开头主题（Wikimedia Commons，
//! https://upload.wikimedia.org/wikipedia/commons/9/90/Erik_Satie_-_gymnopedies_-_la_1_ere._lent_et_douloureux.ogg
//! 许可：Public domain，可自由分发）——裁剪 18s + 响度归一 -18 LUFS + 淡入淡出。

use ffanim::kurbo::{BezPath, Vec2};
use ffanim::typst::{FormulaOptions, TextOptions, TypstEngine};
use ffanim::{Anim, Axes, Camera, Color, Font, Item, RateFunc, Theme, Timeline};

const W: u32 = 960;
const H: u32 = 540;
const FPS: u32 = 30;

fn main() -> ffanim::Result<()> {
    // 数据驱动：颜色与描边宽度来自主题 JSON（仓库根 themes/）
    let theme_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../themes/default.json");
    let theme = Theme::from_path(theme_path)?;
    let accent = theme.accent();
    let fg = theme.foreground();
    let mut tl = Timeline::new(W, H, FPS, theme.background());

    // 排版引擎 + 系统中文字体（env FFANIM_CJK_FONT → fc-list :lang=zh → 常见路径，
    // 无硬编码路径；缺失时 Typst 退回内置拉丁字体）
    let mut engine = TypstEngine::new(16);
    if let Some((font, idx)) = ffanim::typst::find_cjk_font() {
        engine.add_font_face(&font, idx)?;
    }
    // strip 时间轴标注用拉丁字体（与文本排版无关）
    let font = Font::from_system(None::<&str>)?;

    // —— 幕 1：标题淡入 ——
    // ANCHOR: fourier_title
    let title = engine.text("傅里叶级数与方波", &TextOptions::new(52.0, fg))?;
    let title_place = title.fit_transform(f64::from(W) / 2.0, 48.0 + title.height_pt / 2.0, 1.0);
    let title_ids: Vec<_> = title
        .to_items()
        .into_iter()
        .map(|item| tl.add(item.with_transform(title_place)))
        .collect();
    tl.group(
        title_ids.iter().map(|&id| Anim::fade_in(id).during(1.0)),
        0.0,
    );
    tl.wait(0.3);
    // ANCHOR_END: fourier_title

    // —— 幕 2：级数公式逐字形写出 ——
    // ANCHOR: fourier_write
    let fopts = FormulaOptions {
        font_size_pt: 30.0,
        text_color: fg,
    };
    let formula = engine.formula(
        "f(x) = 4/pi (sin x + frac(sin 3x, 3) + frac(sin 5x, 5) + dots)",
        &fopts,
    )?;
    // 缩放后居中于标题下方（fit_transform 修正 center_transform 的缩放偏移）
    let place = formula.fit_transform(f64::from(W) / 2.0, 158.0, 1.5);
    let write_dur = 0.5;
    let step = 0.06;
    // 公式逐字写出：group 按步长错开排入（M8.3 组动画，替代逐段 play+overlap 手工对齐）
    let ids: Vec<_> = formula
        .to_items()
        .into_iter()
        .map(|piece| tl.add(piece.with_transform(place)))
        .collect();
    tl.group(
        ids.iter()
            .map(|&id| Anim::create(id).during(write_dur).rate(RateFunc::Smooth)),
        step,
    );
    tl.wait(0.5);
    // ANCHOR_END: fourier_write

    // —— 幕 3：坐标系 + 方波目标 ——
    // ANCHOR: fourier_axes
    let axes = Axes::new((-3.5, 3.5), (-1.5, 1.5))
        .with_size(880.0, 270.0)
        .with_center(f64::from(W) / 2.0, 350.0);
    let grid = tl.add(axes.grid_item(fg.with_alpha(40), 1.0));
    tl.play(Anim::fade_in(grid).during(0.6));
    let ax = tl.add(axes.to_item(&theme.style));
    tl.overlap(0.4);
    tl.play(Anim::create(ax).during(1.0));
    // 方波目标：sign(sin(πx/2))，细线淡入（半透明前景色）
    let square = tl.add(square_wave_item(&axes, fg.with_alpha(130)));
    tl.overlap(0.6);
    tl.play(Anim::fade_in(square).during(0.8));
    tl.wait(0.3);
    // ANCHOR_END: fourier_axes

    // —— 幕 4：部分和 S_N 随 tracker N 逐阶升高 ——
    // ANCHOR: fourier_tracker
    let sum = tl.add_tracker(1.0);
    let curve = tl.add_updater(&[sum], move |v| {
        let nn = v[0].round() as i64;
        axes.plot(
            |x| {
                // S_N(x) = 4/π · Σ_{k=1,3,..,N} sin(kπx/2)/k
                let mut s = 0.0;
                let mut k = 1;
                while k <= nn {
                    s += (k as f64 * x * std::f64::consts::FRAC_PI_2).sin() / k as f64;
                    k += 2;
                }
                4.0 / std::f64::consts::PI * s
            },
            (-3.5, 3.5),
        )
        .with_stroke(accent)
        .with_stroke_width(3.0)
    });
    tl.play(Anim::create(curve).during(1.5));
    tl.wait(0.3);
    // 逐阶升高：N = 1 → 3 → 5 → 7 → 9（跳变处过冲随之成形，吉布斯现象）
    for target in [3.0, 5.0, 7.0, 9.0] {
        tl.play(Anim::set_value(sum, target).during(1.2));
        tl.wait(0.2);
    }

    // 推近镜头（M8.1 相机系统验收段）：相机即 Item，transform_to 到目标仿射 ——
    let camera = tl.camera(Camera::at(W, H));
    let push_target = Camera {
        center: Vec2::new(f64::from(W) / 2.0, 350.0), // 注视方波曲线中心
        zoom: 1.4,
        rotation: 0.0,
    };
    tl.play(
        Anim::transform_to(camera, push_target.affine(W, H))
            .during(1.4)
            .rate(RateFunc::Smooth),
    );

    // —— 幕 5：结论 ——
    let caption = engine.text(
        "N → ∞ 时逼近方波，跳变处残留 ≈ 9% 过冲（吉布斯现象）",
        &TextOptions::new(26.0, fg),
    )?;
    let cap_place = caption.fit_transform(f64::from(W) / 2.0, 495.0 + caption.height_pt / 2.0, 1.0);
    let cap_ids: Vec<_> = caption
        .to_items()
        .into_iter()
        .map(|item| tl.add(item.with_transform(cap_place)))
        .collect();
    tl.group(cap_ids.iter().map(|&id| Anim::fade_in(id).during(0.8)), 0.0);
    tl.wait(1.2);

    let mut tl = tl.compile();
    // ANCHOR_END: fourier_tracker

    // —— 出片 + 无看片诊断（agent 验证循环的库式用法）——
    // ANCHOR: fourier_render
    // BGM 环境变量指向音频文件时：analyze → 片长延伸到音频长度（FromAudio 语义）
    // → 渲染临时文件 → 混流为成品
    let bgm_path = std::env::var("BGM").ok();
    let encode_out: std::path::PathBuf = match &bgm_path {
        Some(bgm) => {
            let report = ffanim::audio::analyze(std::path::Path::new(bgm))?;
            let frames = (report.duration * f64::from(FPS)).ceil() as usize;
            println!(
                "BGM {bgm}: {:.2}s（RMS {:.1}dBFS）→ 片长延伸到 {frames} 帧",
                report.duration, report.rms_db
            );
            tl = tl.with_min_duration(frames);
            "out/fourier_tmp.mp4".into()
        }
        None => "out/fourier.mp4".into(),
    };
    ffanim::render_to_mp4(&tl, &encode_out)?;
    println!(
        "已输出 {encode_out:?}（{:.2}s）",
        tl.duration_frames as f64 / f64::from(FPS)
    );
    if let Some(bgm) = &bgm_path {
        ffanim::audio::mux(
            std::path::Path::new(&encode_out),
            std::path::Path::new(bgm),
            std::path::Path::new("out/fourier.mp4"),
        )?;
        std::fs::remove_file(&encode_out).ok();
        println!("音轨已混入 out/fourier.mp4");
    }

    let report = ffanim::tools::strip(
        &tl,
        Some(&font),
        accent,
        "out/fourier_strip.png".into(),
        12,
        4,
        320,
    )
    .map_err(ffanim::Error::Other)?;
    println!("strip: {:?}（采样帧 {:?}）", report.out, report.frames);

    // onion 取部分和阶跃段中点（N: 3→5 期间）
    let onion_frame = tl.duration_frames * 2 / 3;
    let report = ffanim::tools::onion(&tl, onion_frame, 6, "out/fourier_onion.png".into())
        .map_err(ffanim::Error::Other)?;
    println!("onion: 基准帧 {} → {:?}", report.frame, report.out);

    Ok(())
}

/// 方波目标曲线：sign(sin(πx/2))，断点 x = -2, 0, 2（数据坐标 → 像素）。
fn square_wave_item(axes: &Axes, color: Color) -> Item {
    let mut path = BezPath::new();
    let pts = [
        (-3.5, -1.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (2.0, 1.0),
        (2.0, -1.0),
        (3.5, -1.0),
    ];
    for (i, (x, y)) in pts.iter().enumerate() {
        let p = axes.c2p(*x, *y);
        if i == 0 {
            path.move_to(p);
        } else {
            path.line_to(p);
        }
    }
    Item::from_path(path)
        .with_stroke(color)
        .with_stroke_width(2.0)
}
