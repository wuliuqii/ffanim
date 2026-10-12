//! 门面 API 集成测试：re-export 覆盖 + 子系统冒烟（docs/02 规则 B：门面 API 必须被测试引用）。
//!
//! ffmpeg 依赖用例（出片/音频）在无 ffmpeg 环境下打点跳过（CI 容器可能未装）；
//! 其余用例（typst/主题/坐标/诊断纯函数/工具）不依赖外部进程，无条件运行。

use std::path::PathBuf;

use ffanim::kurbo::Circle;
use ffanim::tools::{OnionReport, StripReport, downsample, onion, sample_frames, strip};
use ffanim::typst::{FormulaOptions, TextOptions, TypstEngine, find_cjk_font, match_formulas};
use ffanim::{Anim, Color, Item, Theme, Timeline, VERSION, render_to_mp4, render_to_mp4_frames};
use ffanim_audio::analyze_pcm;

const BG: Color = Color::new(14, 17, 22, 255);
const FG: Color = Color::new(230, 233, 239, 255);

/// 仓库根（crates/ffanim → 上两级）。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../")
}

/// 本次测试的临时输出目录（进程级唯一，不清理便于排障）。
fn out_dir(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("ffanim-api-{}-{tag}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

/// ffmpeg 是否可用（缺失时用例打点跳过）。
fn has_ffmpeg() -> bool {
    std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// 最小时间轴：淡入一个圆（1s @30fps，30 帧）。
fn small_timeline() -> Timeline {
    let mut t = Timeline::new(96, 54, 30, BG);
    let a = t.add(
        Item::circle(Circle::new((48.0, 27.0), 10.0)).with_fill(Color::new(80, 160, 255, 255)),
    );
    t.play(Anim::fade_in(a).during(1.0));
    t
}

#[test]
fn version_与包版本一致() {
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
}

#[test]
fn typst_公式排版与结构匹配() {
    let mut eng = TypstEngine::new(8);
    // 显式类型标注：Formula / FormulaOptions 均为门面公开项（规则 B）
    let a: std::sync::Arc<ffanim::typst::Formula> = eng
        .formula(
            "x^2 + 1",
            &FormulaOptions {
                font_size_pt: 32.0,
                text_color: FG,
            },
        )
        .expect("公式排版不应失败");
    let b = eng
        .formula(
            "x^2 - 1",
            &FormulaOptions {
                font_size_pt: 32.0,
                text_color: FG,
            },
        )
        .expect("公式排版不应失败");
    assert!(!a.pieces.is_empty(), "公式 A 应有绘制片段");
    assert!(a.width_pt > 0.0 && a.height_pt > 0.0, "公式应有正尺寸");

    // LCS 匹配：`x^2` 与 `1` 应配对，`+`/`-` 各自落单
    let plan = match_formulas(&a, &b);
    assert!(!plan.paired.is_empty(), "相同子式应配对: {plan:?}");
    assert_eq!(plan.leave_a.len(), 1, "A 落单片段 = '+");
    assert_eq!(plan.enter_b.len(), 1, "B 落单片段 = '-");
}

#[test]
fn typst_文本排版冒烟() {
    let mut eng = TypstEngine::new(8);
    let f = eng
        .text("ffanim", &TextOptions::new(24.0, FG))
        .expect("文本排版不应失败（内嵌 Latin 字体）");
    assert!(!f.pieces.is_empty(), "文本应有字形片段");
    assert!(f.width_pt > 0.0);
    // CJK 字体查找：无字体环境返回 None 也合法（不强制 Some）
    let _ = find_cjk_font();
}

#[test]
fn theme_默认主题加载() {
    let theme =
        Theme::from_path(repo_root().join("themes/default.json")).expect("默认主题应可加载");
    assert!(!theme.name.is_empty());
    // 语义色三件套均可解析
    theme.background().expect("背景色");
    theme.foreground().expect("前景色");
    theme.accent().expect("强调色");
    // Style 为门面公开项（规则 B）：对象默认样式随主题加载
    let _style: &ffanim::Style = &theme.style;
}

#[test]
fn axes_坐标往返一致() {
    let ax = ffanim::Axes::new((0.0, 10.0), (0.0, 5.0)).with_size(100.0, 50.0);
    let p = ax.c2p(5.0, 2.5);
    let (x, y) = ax.p2c(p);
    assert!(
        (x - 5.0).abs() < 1e-9 && (y - 2.5).abs() < 1e-9,
        "c2p→p2c 往返: ({x}, {y})"
    );
}

#[test]
fn sample_frames_等距采样() {
    let frames = sample_frames(90, 4);
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0], 0, "首帧含入");
    assert_eq!(*frames.last().unwrap(), 89, "末帧含入");
    assert!(frames.windows(2).all(|w| w[0] < w[1]), "采样应严格递增");
}

#[test]
fn downsample_缩略尺寸() {
    // 4x2 RGBA → 2x1：输出 2*1*4 = 8 字节
    let src: Vec<u8> = (0..4 * 2 * 4).map(|i| (i % 251) as u8).collect();
    let dst = downsample(&src, 4, 2, 2, 1);
    assert_eq!(dst.len(), 8);
}

#[test]
fn strip_接触表报告() {
    let ct = small_timeline().compile();
    let out = out_dir("strip").join("strip.png");
    let report: StripReport = strip(
        &ct,
        None,
        Color::new(255, 200, 80, 255),
        out.clone(),
        4,
        2,
        96,
    )
    .expect("strip 不依赖 ffmpeg，应成功");
    assert_eq!(report.frames.len(), 4, "采样 4 帧");
    assert_eq!(report.cols, 2, "每行 2 列");
    assert_eq!(report.thumb.0, 96, "缩略宽 = thumb_w");
    assert!(out.is_file(), "接触表 PNG 应落盘");
}

#[test]
fn onion_洋葱皮报告() {
    let ct = small_timeline().compile();
    let out = out_dir("onion").join("onion.png");
    let report: OnionReport = onion(&ct, 15, 2, out.clone()).expect("onion 不依赖 ffmpeg，应成功");
    assert_eq!(report.frame, 15);
    assert_eq!(report.spread, 2);
    assert!(out.is_file(), "洋葱皮 PNG 应落盘");
}

#[test]
fn render_to_mp4_出片与音频混流冒烟() {
    if !has_ffmpeg() {
        eprintln!("跳过：环境无 ffmpeg");
        return;
    }
    let dir = out_dir("av");

    // 视频：门面一键出片
    let mp4 = dir.join("api_smoke.mp4");
    render_to_mp4(&small_timeline().compile(), &mp4).expect("CPU 渲染 + ffmpeg 编码应成功");
    assert!(
        mp4.metadata().map(|m| m.len() > 0).unwrap_or(false),
        "mp4 应非空"
    );

    // 音频：生成 1s 正弦 wav → analyze 诊断 → 混流入片
    let wav = dir.join("sine.wav");
    let status = std::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=1",
            "-ar",
            "48000",
            "-ac",
            "1",
        ])
        .arg(&wav)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("启动 ffmpeg");
    assert!(status.success(), "生成 wav 失败");

    // AudioReport 为门面公开项（规则 B）：显式类型标注
    let report: ffanim::audio::AudioReport = ffanim::audio::analyze(&wav).expect("wav 诊断应成功");
    assert!(
        (report.duration - 1.0).abs() < 0.1,
        "时长 ≈1s: {}",
        report.duration
    );
    assert_eq!(report.sample_rate, 48000);
    assert_eq!(report.channels, 1);

    let av = dir.join("api_smoke_av.mp4");
    ffanim::audio::mux(&mp4, &wav, &av).expect("混流应成功");
    assert!(av.is_file(), "混流产物应落盘");
}

#[test]
fn render_to_mp4_frames_片段导出冒烟() {
    if !has_ffmpeg() {
        eprintln!("跳过：环境无 ffmpeg");
        return;
    }
    let dir = out_dir("section");
    let mp4 = dir.join("section_10_25.mp4");
    // 30 帧全片只导出 [10, 25) = 15 帧（0.5s）
    render_to_mp4_frames(&small_timeline().compile(), &mp4, 10..25).expect("片段导出应成功");
    assert!(
        mp4.metadata().map(|m| m.len() > 0).unwrap_or(false),
        "片段 mp4 应非空"
    );
    // 越界区间报错（全片 30 帧，25..40 越界）
    let err = render_to_mp4_frames(&small_timeline().compile(), dir.join("bad.mp4"), 25..40);
    assert!(err.is_err(), "越界区间应报错: {err:?}");
}

#[test]
fn analyze_pcm_纯函数诊断() {
    // 全零：无削波
    let silent = analyze_pcm(&[0.0; 480], 48_000);
    assert_eq!(silent.clipped_samples, 0);
    // 超限样本计入削波
    let hot = analyze_pcm(&[0.0, 2.0, -3.0], 48_000);
    assert_eq!(
        hot.clipped_samples, 2,
        "|x|>1 计削波: {:?}",
        hot.clipped_samples
    );
}

#[test]
fn primitives_标注基元与位姿动作() {
    use ffanim::PoseDelta;
    use ffanim::kurbo::{Affine, Point, Vec2};

    // —— 标注基元：路径非空 + 包围盒合理 ——
    assert!((Item::line((0.0, 0.0), (100.0, 0.0)).bbox().x1 - 100.0).abs() < 1e-9);
    let arrow = Item::arrow((0.0, 0.0), (100.0, 0.0));
    assert!(
        arrow.bbox().y0 < 0.0,
        "箭头头部应超出线段包围盒: {:?}",
        arrow.bbox()
    );
    assert!(
        (Item::dashed_line((0.0, 0.0), (50.0, 0.0), 10.0, 5.0)
            .bbox()
            .x1
            - 50.0)
            .abs()
            < 1e-9
    );
    let ell = Item::ellipse((0.0, 0.0), 30.0, 10.0);
    assert!((ell.bbox().width() - 60.0).abs() < 1.0, "{:?}", ell.bbox());
    assert!(
        Item::arc((0.0, 0.0), 50.0, 0.0, 90.0).bbox().y0 < 0.0,
        "视觉逆时针弧应在上方"
    );

    // —— 位姿动作：构建期 Pose 在 play 时解析为绝对 TransformTo ——
    let mut tl = Timeline::new(200, 200, 30, BG);
    let id = tl.add(arrow);
    tl.play(Anim::shift_to(id, 40.0, 0.0).during(1.0));
    tl.play(Anim::rotate_to(id, std::f64::consts::FRAC_PI_2).during(1.0));
    tl.play(Anim::scale_to(id, 2.0).during(1.0));
    tl.play(Anim::move_to(id, (150.0, 150.0)).during(1.0));
    let c = tl.compile();
    assert_eq!(c.anims.len(), 4, "四段位姿动作全部落表");
    assert!(
        c.anims
            .iter()
            .all(|a| matches!(a.kind, ffanim::AnimKind::TransformTo { .. })),
        "编译表内不应残留 Pose 变体"
    );

    // PoseDelta 为门面公开项（规则 B）：resolve 语义直接断言
    let d: PoseDelta = PoseDelta::Shift(Vec2::new(40.0, 0.0));
    let to = d.resolve(Affine::IDENTITY, Point::new(50.0, 25.0));
    let p = to * Point::new(50.0, 25.0);
    assert!(
        (p.x - 90.0).abs() < 1e-9 && (p.y - 25.0).abs() < 1e-9,
        "{p:?}"
    );
}

#[test]
fn layout_相对布局与画布贴边() {
    use ffanim::kurbo::Rect;
    use ffanim::{Direction, arrange};

    let base = Item::rect(Rect::new(0.0, 0.0, 100.0, 60.0));
    let other = Item::rect(Rect::new(300.0, 100.0, 400.0, 160.0));

    // next_to 右侧：x0 = other.x1 + buff，y 中心对齐
    let c = base.clone().next_to(&other, Direction::Right, 20.0);
    let bb = c.bbox();
    assert!((bb.x0 - 420.0).abs() < 1e-9, "{bb:?}");
    assert!((bb.y0 - 100.0).abs() < 1e-9, "垂直居中: {bb:?}");

    // to_edge 顶部：y0 = buff，x 对画布居中
    let t = base.clone().to_edge(Direction::Up, 10.0, (960.0, 540.0));
    let bb = t.bbox();
    assert!(
        (bb.y0 - 10.0).abs() < 1e-9 && (bb.x0 - 430.0).abs() < 1e-9,
        "{bb:?}"
    );

    // arrange 横排：总包围盒含两项与间距
    let mut row = vec![base.clone(), base];
    let total: ffanim::kurbo::Rect = arrange(&mut row, Direction::Right, 30.0);
    assert!(
        (total.x1 - 230.0).abs() < 1e-9 && (total.y1 - 60.0).abs() < 1e-9,
        "{total:?}"
    );
}

#[test]
fn emphasize_强调动画编排() {
    use ffanim::AnimKind;
    use ffanim::kurbo::{Affine, Circle, Point};

    let mut tl = Timeline::new(200, 200, 30, BG);
    let circle = tl.add(Item::circle(Circle::new((50.0, 50.0), 30.0)).with_stroke(FG));
    tl.play(Anim::create(circle).during(1.0)); // 段 0：[0,30)
    tl.indicate(circle, 1.2, 1.0); // 段 1/2：[30,45)/[45,60)
    tl.circumscribe(circle, 0.8, 10.0, Color::new(255, 200, 0, 255)); // 段 3/4：[60,72)/[72,84)
    tl.flash((100.0, 100.0), 0.6, 20.0, Color::new(255, 200, 0, 255)); // 段 5/6/7：[84,93)/[84,102)/[93,102)
    tl.show_passing_flash(circle, 0.0, 0.5, 0.4, Color::new(255, 200, 0, 255)); // 段 8/9：[102,108)/[108,114)
    let c = tl.compile();
    assert_eq!(c.anims.len(), 10, "四件套展开 = 1+2+2+3+2 段");
    assert_eq!(c.duration_frames, 114);

    // 时序：indicate 两段 / circumscribe 两段 / flash 三段同窗 / passing 两段
    let w = |i: usize| (c.anims[i].start, c.anims[i].end);
    assert_eq!(w(1), (30, 45), "indicate 放大段");
    assert_eq!(w(2), (45, 60), "indicate 复原段");
    assert_eq!(w(3), (60, 72), "circumscribe 框淡入");
    assert_eq!(w(4), (72, 84), "circumscribe 框淡出");
    assert_eq!(w(5), (84, 93), "flash 淡入前半");
    assert_eq!(w(6), (84, 102), "flash 放大全程");
    assert_eq!(w(7), (93, 102), "flash 淡出后半");
    assert_eq!(w(8), (102, 108), "passing 淡入");
    assert_eq!(w(9), (108, 114), "passing 淡出");

    // indicate 峰值 = 绕圆心放大 1.2（复原段首帧），段后复位原位
    let expect =
        Affine::translate((50.0, 50.0)) * Affine::scale(1.2) * Affine::translate((-50.0, -50.0));
    let mid = ffanim::render_frame(&c, 45);
    assert!(mid.items.len() == 1, "indicate 段间只应剩圆本身");
    let t = mid.items[0].transform;
    let p = t * Point::new(50.0, 50.0);
    let q = expect * Point::new(50.0, 50.0);
    assert!(
        (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6,
        "峰值应为绕圆心放大: {t:?}"
    );

    // 临时对象可见性：框/闪环/高亮段段前不可见、中段可见、段后隐藏
    assert_eq!(
        ffanim::render_frame(&c, 65).items.len(),
        2,
        "框淡入中 = 圆+框"
    );
    assert_eq!(ffanim::render_frame(&c, 114).items.len(), 1, "收尾只余圆");

    // flash 放大段落表为 TransformTo（构建期编排不残留新变体）
    assert!(
        matches!(c.anims[6].kind, AnimKind::TransformTo { .. }),
        "flash 放大段应为 TransformTo"
    );
}

#[test]
fn sugar_薄糖批编排() {
    use ffanim::kurbo::{Circle, Rect};

    let mut tl = Timeline::new(200, 200, 30, BG);

    // 1) Uncreate：create → uncreate，段后对象隐藏
    let circle = tl.add(Item::circle(Circle::new((50.0, 50.0), 20.0)).with_stroke(FG));
    tl.play(Anim::create(circle).during(0.5));
    tl.play(Anim::uncreate(circle).during(0.5));

    // 2) Together：不同时长同窗对齐，游标取最长
    let a = tl.add(Item::rect(Rect::new(0.0, 0.0, 30.0, 20.0)).with_fill(FG));
    let b = tl.add(Item::rect(Rect::new(100.0, 0.0, 30.0, 20.0)).with_fill(FG));
    tl.together([Anim::fade_in(a).during(0.5), Anim::fade_in(b).during(1.0)]);

    // 3) Succession：依次接续
    let c1 = tl.add(Item::rect(Rect::new(0.0, 100.0, 20.0, 20.0)).with_fill(FG));
    tl.succession([
        Anim::fade_in(c1).during(0.4),
        Anim::fade_out(c1).during(0.4),
    ]);

    // 4) 方向家族落表冒烟（FadeIn 变体参数化）
    let d = tl.add(Item::rect(Rect::new(100.0, 100.0, 20.0, 20.0)).with_fill(FG));
    tl.play(Anim::grow_from_center(d).during(0.5));
    tl.play(Anim::spin_in(d, std::f64::consts::FRAC_PI_2).during(0.5));

    // 5) save_state / restore：move_to 后 morph 回快照，返回新句柄
    let snap = tl.save_state(a);
    tl.play(Anim::move_to(a, (150.0, 150.0)).during(0.5));
    let restored = tl.restore(a, &snap);

    let ct = tl.compile();
    // 1+1 / 2 / 2 / 2 / 1+1 = 10 段
    assert_eq!(ct.anims.len(), 10, "{:?}", ct.anims.len());
    // together 同窗
    assert_eq!((ct.anims[2].start, ct.anims[3].start), (30, 30));
    // restore 返回的句柄对象在场且复位（morph 段结束后）
    let after = ffanim::render_frame(&ct, ct.duration_frames);
    assert!(
        after.items.iter().any(|it| it.id == restored),
        "恢复句柄应在场"
    );
    let snap_t = after
        .items
        .iter()
        .find(|it| it.id == restored)
        .unwrap()
        .transform;
    assert_eq!(snap_t, snap.transform, "应复位到快照变换");
    // uncreate 段后圆隐藏
    assert!(
        !after.items.iter().any(|it| it.id == circle),
        "uncreate 后圆应退场"
    );
}
