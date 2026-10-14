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

/// P1-7 标注复合件编排：四构造器基于目标 bbox 几何 + 拼组标注 + 圈选框随目标同步位姿。
#[test]
fn annotate_标注复合件编排() {
    use ffanim::kurbo::Rect;
    use ffanim::render_frame;

    let mut tl = Timeline::new(480, 270, 30, BG);

    // 1) 目标矩形 + 圈选框（外扩 10）：框几何应等于目标 bbox ± buff
    let target = tl.add(Item::rect(Rect::new(100.0, 50.0, 200.0, 100.0)));
    let grp = Item::group(vec![Item::rect(Rect::new(100.0, 50.0, 200.0, 100.0))]);
    let frame = tl.add(Item::surrounding_rect(&grp, 10.0).with_stroke(FG));
    tl.play(Anim::create(frame).during(0.5));

    // 2) 下划线：底部下方 8px 等宽横线
    let ul = tl.add(Item::underline(&grp, 8.0).with_stroke(FG));
    tl.play(Anim::create(ul).during(0.4));

    // 3) 叉 + 背景板（注册先于目标 = 画在下）
    let cr = tl.add(Item::cross(&grp, 6.0).with_stroke(FG));
    tl.play(Anim::create(cr).during(0.4));
    let bg = tl.add(Item::background_rect(&grp, 12.0).with_fill(BG));

    // 4) 圈选框随目标同步平移（标注基线语义）
    tl.together([
        Anim::shift_to(target, 40.0, 0.0).during(0.5),
        Anim::shift_to(frame, 40.0, 0.0).during(0.5),
    ]);

    let ct = tl.compile();
    // 1 / 1 / 1 / 2 = 5 段
    assert_eq!(ct.anims.len(), 5, "{:?}", ct.anims.len());
    // together 同窗（段 3、4）
    assert_eq!((ct.anims[3].start, ct.anims[4].start), (39, 39));

    // 几何断言：compile 后基线已推进到 to 态（目标与框同步右移 40），
    // 圈选框仍罩住目标 bbox ± 10（目标 (140,50)~(240,100)）
    let frame_item = &ct.items[frame as usize];
    assert_eq!(frame_item.bbox(), Rect::new(130.0, 40.0, 250.0, 110.0));
    // 下划线 = 底部（y1=100）下方 8px 等宽线
    let ul_bb = ct.items[ul as usize].bbox();
    assert_eq!(ul_bb, Rect::new(100.0, 108.0, 200.0, 108.0), "{ul_bb:?}");
    // 背景板 = 外扩 12
    assert_eq!(
        ct.items[bg as usize].bbox(),
        Rect::new(88.0, 38.0, 212.0, 112.0)
    );

    // 同步平移后（段末）：框与目标一起右移 40
    let after = render_frame(&ct, ct.duration_frames);
    let frame_t = after
        .items
        .iter()
        .find(|it| it.id == frame)
        .unwrap()
        .transform;
    assert_eq!(
        frame_t,
        ffanim::kurbo::Affine::translate((40.0, 0.0)),
        "{frame_t:?}"
    );
}

/// P1-13 代码块高亮编排：engine.code（门面 re-export CodeOptions/CodeTheme）
/// + 逐字形写出 + surrounding_rect 圈选。
#[test]
fn code_代码块高亮编排() {
    use ffanim::kurbo::Rect;
    use ffanim::typst::{CodeOptions, CodeTheme};

    let mut eng = TypstEngine::new(8);
    let src = "fn main() { let x = 1; }";
    let opts = CodeOptions::new(13.0);
    let f = eng.code(src, "rust", &opts).unwrap();

    // 多色高亮：至少 3 种 token 色，含深色主题前景色（piece fill 为小写 hex）
    let fills: std::collections::HashSet<String> = f
        .pieces
        .iter()
        .filter_map(|p| {
            p.fill
                .map(|c| format!("#{:02x}{:02x}{:02x}", c.r, c.g, c.b))
        })
        .collect();
    assert!(fills.len() >= 3, "应至少 3 种 token 色，实际 {fills:?}");
    assert_eq!(CodeTheme::Dark.foreground_hex().to_lowercase(), "#e6edf3");
    assert!(
        fills.contains(&CodeTheme::Dark.foreground_hex().to_lowercase()),
        "应含主题前景色，实际 {fills:?}"
    );

    // 编排：代码块淡入 → 圈选框描边生长（框应罩住代码 bbox ± buff）
    let mut tl = Timeline::new(960, 540, 30, BG);
    let scale = 2.0;
    let place = f.fit_transform(480.0, 270.0, scale);
    let pieces: Vec<Item> = f
        .to_items()
        .into_iter()
        .map(|it| it.with_transform(place))
        .collect();
    let grp = Item::group(pieces.clone());
    let ids = tl.add_group(pieces);
    tl.group(ids.iter().map(|&id| Anim::fade_in(id).during(0.8)), 0.0);
    let frame = tl.add(Item::surrounding_rect(&grp, 20.0).with_stroke(FG));
    tl.play(Anim::create(frame).during(0.6));

    let ct = tl.compile();
    assert_eq!(ct.anims.len(), ids.len() + 1);

    // 框 bbox = 代码组 bbox ± 20（标注基于注册基线，fade_in 不改位姿）
    let code_bb = grp.bbox();
    let frame_bb = ct.items[frame as usize].bbox();
    assert_eq!(
        frame_bb,
        Rect::new(
            code_bb.x0 - 20.0,
            code_bb.y0 - 20.0,
            code_bb.x1 + 20.0,
            code_bb.y1 + 20.0
        ),
        "代码 {code_bb:?} vs 框 {frame_bb:?}"
    );
}

/// P1-4 沿路径移动编排：`Anim::move_along_path` 弧长匀速采样
/// + prepare 基线推进到路径终点 + 段中逐帧求值。
#[test]
fn move_沿路径移动编排() {
    use ffanim::kurbo::{Affine, BezPath, Circle};
    use ffanim::{RateFunc, render_frame};

    let mut tl = Timeline::new(960, 540, 30, BG);
    // 折线轨迹：(100,400) → (300,400) → (300,200)，两段各 200pt，总弧长 400
    let mut track = BezPath::new();
    track.move_to((100.0, 400.0));
    track.line_to((300.0, 400.0));
    track.line_to((300.0, 200.0));
    let ball = tl.add(Item::circle(Circle::new((100.0, 400.0), 12.0)).with_fill(FG));
    // 显式 Linear：隔离缓动变量，纯验证弧长匀速采样
    tl.play(
        Anim::move_along_path(ball, &track)
            .during(1.0)
            .rate(RateFunc::Linear),
    );

    let ct = tl.compile();
    assert_eq!(ct.duration_frames, 30);
    // prepare 基线快进到路径终点偏移 (200,-200)（后续动画以此为接续基线）
    let ball_t = ct.items[ball as usize].transform;
    assert_eq!(ball_t, Affine::translate((200.0, -200.0)), "{ball_t:?}");

    // 段中求值：弧长比例采样（Linear 缓动）
    // frame 15（alpha=0.5）走完第一段 → 偏移 (100,0)；
    // frame 22（alpha=22/30）第二段走 93.3/200 → 偏移 (200,-93.3)；
    // frame 30（窗口外）→ 基线终点 (200,-200)
    let tx = |f: usize| {
        render_frame(&ct, f).items[ball as usize]
            .transform
            .as_coeffs()
    };
    let start = tx(0);
    assert!(
        start[4].abs() < 1e-9 && start[5].abs() < 1e-9,
        "起点偏移 {start:?}"
    );
    let mid = tx(15);
    assert!(
        (mid[4] - 200.0).abs() < 1e-6 && mid[5].abs() < 1e-6,
        "中点偏移（第一段末）= ({},{})",
        mid[4],
        mid[5]
    );
    let q4 = tx(7); // alpha=7/30 → 第一段走 93.3/200 → 偏移 (93.3,0)
    assert!(
        (q4[4] - 7.0 / 30.0 * 400.0).abs() < 1e-6 && q4[5].abs() < 1e-6,
        "四分位偏移 = ({},{})",
        q4[4],
        q4[5]
    );
    let q = tx(22);
    let ey = -(22.0_f64 / 30.0 * 400.0 - 200.0);
    assert!(
        (q[4] - 200.0).abs() < 1e-6 && (q[5] - ey).abs() < 1e-6,
        "22 帧偏移 = ({},{})，期望 y={ey}",
        q[4],
        q[5]
    );
    let end = tx(30);
    assert!(
        (end[4] - 200.0).abs() < 1e-9 && (end[5] + 200.0).abs() < 1e-9,
        "段后偏移 = ({},{})",
        end[4],
        end[5]
    );
}

/// P1-5 结构匹配形变编排（公式以外）：`match_shapes` 几何签名配对 +
/// `MatchPlan` 调度 transform（门面 re-export 覆盖）。
#[test]
fn shapes_结构匹配形变编排() {
    use ffanim::kurbo::{Affine, Circle, Rect};
    use ffanim::{MatchPlan, match_shapes, render_frame};

    // 三件套：同 base path + transform 挂位（验证平移无关配对）
    let sq = || Item::rect(Rect::new(0.0, 0.0, 60.0, 60.0));
    let ci = || Item::circle(Circle::new((0.0, 0.0), 30.0));
    let tri = || Item::regular_polygon((0.0, 0.0).into(), 30.0, 3);
    let place = |it: Item, x: f64| {
        it.with_fill(FG)
            .with_transform(Affine::translate((x, 270.0)))
    };
    // a = [方, 三角, 圆] → b = [方, 圆, 三角]：后两者交换（乱序移动）
    let a = vec![place(sq(), 100.0), place(tri(), 200.0), place(ci(), 300.0)];
    let b = vec![place(sq(), 500.0), place(ci(), 600.0), place(tri(), 700.0)];

    let plan: MatchPlan = match_shapes(&a, &b);
    assert_eq!(plan.paired, vec![(0, 0), (1, 2), (2, 1)], "{plan:?}");
    assert!(plan.leave_a.is_empty() && plan.enter_b.is_empty());

    // 编排：a 同窗淡入 → 配对形变（together 同窗口）
    let mut tl = Timeline::new(960, 540, 30, BG);
    let ids_a: Vec<_> = a.iter().map(|it| tl.add(it.clone())).collect();
    let ids_b: Vec<_> = b.iter().map(|it| tl.add(it.clone())).collect();
    tl.group(ids_a.iter().map(|&id| Anim::fade_in(id).during(0.6)), 0.0);
    tl.wait(0.3);
    tl.together(
        plan.paired
            .iter()
            .map(|&(i, j)| Anim::transform(ids_a[i], ids_b[j]).during(1.0)),
    );
    let ct = tl.compile();
    assert_eq!(ct.anims.len(), 6, "3 淡入 + 3 形变");
    assert_eq!(ct.duration_frames, 57, "0.6 + 0.3 + 1.0 @30fps");

    // 求值：形变窗口前 a 可见（3 件）→ 窗口中形变插值（3 件）→ 窗口后 b 可见（3 件）
    assert_eq!(render_frame(&ct, 20).items.len(), 3);
    assert_eq!(render_frame(&ct, 42).items.len(), 3);
    assert_eq!(render_frame(&ct, 56).items.len(), 3);
    // 窗口中点：三角从 a1(200,270) 形变到 b2(700,270)，DrawList 位置取插值中点
    let morph = render_frame(&ct, 42);
    let tri_tx = morph.items[1].transform.as_coeffs();
    assert!(
        (tri_tx[4] - 450.0).abs() < 1.0,
        "三角中点 x = {}",
        tri_tx[4]
    );
}

#[test]
fn brace_花括号括注编排() {
    use ffanim::kurbo::Rect;
    use ffanim::typst::TypstEngine;
    use ffanim::{Direction, render_frame};

    // 目标横条 + 下方花括号 + 括注文本（与 scenes.rs brace_demo 同构）
    let bar = Item::rect(Rect::new(200.0, 200.0, 600.0, 320.0)).with_fill(FG);
    // 无系统字体的环境跳过（与 typst 单测同一兜底模式）
    let Some((path, idx)) = ffanim_core::text::find_system_font(&[]) else {
        eprintln!("无系统字体，跳过");
        return;
    };
    let font = ffanim::Font::from_index(&path, idx).expect("加载字体");
    let mut engine = TypstEngine::new(8);
    let (brace, label) = engine
        .brace_with_text(&bar, Direction::Down, 12.0, &font, "宽度", 30.0)
        .expect("括注文本构造");
    // 几何：锚点 = (x0, y1+buff)，深度 = 1.2 × clamp(span×0.09, 8, 18) = 1.2×18
    let bb = brace.bbox();
    assert_eq!(bb.x0, 200.0, "左端对齐目标左边");
    assert_eq!(bb.x1, 600.0, "右端对齐目标右边");
    assert!((bb.y0 - 332.0).abs() < 0.5, "开口贴目标下沿 + buff");
    assert!((bb.y1 - 353.6).abs() < 1.0, "尖角深度 21.6，实际 {bb:?}");

    // 编排：横条生长 → 花括号淡入放大（manim GrowFromCenter 对应）+ 文本淡入同窗
    let mut tl = Timeline::new(960, 540, 30, BG);
    let bar_id = tl.add(bar);
    tl.play(Anim::create(bar_id).during(0.8));
    let (bid, lid) = (tl.add(brace), tl.add(label));
    tl.wait(0.2);
    tl.together([
        Anim::fade_in(bid).during(1.0),
        Anim::fade_in(lid).during(0.9),
    ]);
    let ct = tl.compile();
    assert_eq!(ct.anims.len(), 3, "1 生长 + together 2 条");
    assert_eq!(ct.duration_frames, 60, "0.8 + 0.2 + 1.0 @30fps");

    // 求值：brace 窗口前 1 件 → 同窗中 3 件 → 段后 3 件
    assert_eq!(render_frame(&ct, 29).items.len(), 1);
    assert_eq!(render_frame(&ct, 45).items.len(), 3);
    assert_eq!(render_frame(&ct, 59).items.len(), 3);
}

#[test]
fn delimiter_四类定界符编排() {
    use ffanim::kurbo::Rect;
    use ffanim::typst::Delimiter;
    use ffanim::{Direction, render_frame};

    // 目标横条 + 四类定界符（与 scenes.rs delimiter_demo 同构）
    let bar = Item::rect(Rect::new(200.0, 200.0, 600.0, 320.0)).with_fill(FG);
    // 无系统字体的环境跳过（与 typst 单测同一兜底模式）
    // 无系统字体的环境跳过（与 typst 单测同一兜底模式）
    if ffanim_core::text::find_system_font(&[]).is_none() {
        eprintln!("无系统字体，跳过");
        return;
    }
    let mut engine = TypstEngine::new(8);

    // 几何：brace 下方（锚 = (x0, y1+buff)，深度 = 1.2×clamp(span×0.09,8,18) = 1.2×18）
    let brace = engine
        .delimiter(&bar, Delimiter::Brace, Direction::Down, 12.0)
        .expect("brace 构造");
    let bb = brace.bbox();
    assert_eq!(bb.x0, 200.0, "左端对齐目标左边");
    assert_eq!(bb.x1, 600.0, "右端对齐目标右边");
    assert!((bb.y0 - 332.0).abs() < 0.5, "开口贴目标下沿 + buff");
    assert!((bb.y1 - 353.6).abs() < 1.0, "尖角深度 21.6，实际 {bb:?}");

    // 其余三类构造冒烟（四方向各一）
    let bracket = engine
        .delimiter(&bar, Delimiter::Bracket, Direction::Up, 12.0)
        .expect("bracket 构造");
    let paren = engine
        .delimiter(&bar, Delimiter::Paren, Direction::Left, 12.0)
        .expect("paren 构造");
    let dbl = engine
        .delimiter(&bar, Delimiter::DoubleBracket, Direction::Right, 12.0)
        .expect("双线构造");
    // bracket 上方：开口贴目标上沿 − buff（Up = rotate(π)，锚 (x1, y0−buff)）
    let ub = bracket.bbox();
    assert!((ub.y1 - 188.0).abs() < 0.5, "开口贴目标上沿 − buff {ub:?}");
    for (name, d) in [("paren", &paren), ("双线", &dbl)] {
        let b = d.bbox();
        assert!(b.width() > 0.0 && b.height() > 0.0, "{name} 非空 {b:?}");
    }

    // 编排：横条生长 → brace/bracket 同窗淡入（manim GrowFromCenter 对应）
    let mut tl = Timeline::new(960, 540, 30, BG);
    let bar_id = tl.add(bar);
    tl.play(Anim::create(bar_id).during(0.8));
    let (bid, kid) = (tl.add(brace), tl.add(bracket));
    tl.wait(0.2);
    tl.together([
        Anim::fade_in(bid).during(1.0),
        Anim::fade_in(kid).during(1.0),
    ]);
    let ct = tl.compile();
    assert_eq!(ct.anims.len(), 3, "1 生长 + together 2 条");
    assert_eq!(ct.duration_frames, 60, "0.8 + 0.2 + 1.0 @30fps");

    // 求值：brace 窗口前 1 件 → 同窗中 3 件 → 段后 3 件
    assert_eq!(render_frame(&ct, 29).items.len(), 1);
    assert_eq!(render_frame(&ct, 45).items.len(), 3);
    assert_eq!(render_frame(&ct, 59).items.len(), 3);
}

#[test]
fn arrow_两点式编排() {
    use ffanim::kurbo::Point;
    use ffanim::render_frame;
    use ffanim::typst::ArrowKind;

    // 无系统字体的环境跳过（与 delimiter 测试同一兜底模式）
    if ffanim_core::text::find_system_font(&[]).is_none() {
        eprintln!("无系统字体，跳过");
        return;
    }
    let mut engine = TypstEngine::new(8);

    // 几何：水平单头（span 400 → 头 24 封顶，±12 对称张开）
    let from = Point::new(100.0, 200.0);
    let to = Point::new(500.0, 200.0);
    let arrow = engine.arrow(from, to, ArrowKind::Single).expect("单头构造");
    let bb = arrow.bbox();
    assert!((bb.x0 - 100.0).abs() < 0.5, "尾端贴 from {bb:?}");
    assert!((bb.x1 - 500.0).abs() < 0.5, "头尖贴 to {bb:?}");
    assert!((bb.height() - 24.0).abs() < 0.5, "头高 24 封顶 {bb:?}");
    assert!((bb.y0 + bb.y1 - 400.0).abs() < 0.5, "杆过弦中线 y=200");

    // 双头 + 斜向构造冒烟
    let dbl = engine.arrow(from, to, ArrowKind::Double).expect("双头构造");
    let dbb = dbl.bbox();
    assert!(
        dbb.width() > 0.0 && dbb.height() > 0.0,
        "双头 bbox 非空 {dbb:?}"
    );
    let diag = engine
        .arrow(
            Point::new(100.0, 100.0),
            Point::new(400.0, 300.0),
            ArrowKind::Single,
        )
        .expect("斜向构造");
    let db = diag.bbox();
    assert!(
        db.width() > 0.0 && db.height() > 0.0,
        "斜向 bbox 非空 {db:?}"
    );

    // 跨度过短报错
    let short = engine.arrow(
        Point::new(0.0, 0.0),
        Point::new(10.0, 0.0),
        ArrowKind::Single,
    );
    assert!(short.is_err(), "跨度 10px 低于下限应报错");

    // 编排：目标淡入 → 箭头淡入（填充字形禁 create，统一 fade_in）
    let mut tl = Timeline::new(960, 540, 30, BG);
    let target = tl.add(Item::circle(Circle::new((100.0, 200.0), 20.0)).with_fill(FG));
    tl.play(Anim::fade_in(target).during(0.8));
    let aid = tl.add(arrow);
    tl.wait(0.2);
    tl.play(Anim::fade_in(aid).during(1.0));
    let ct = tl.compile();
    assert_eq!(ct.anims.len(), 2, "1 目标淡入 + 1 箭头淡入");
    assert_eq!(ct.duration_frames, 60, "0.8 + 0.2 + 1.0 @30fps");

    // 求值：目标窗口前 1 件 → 同窗中 2 件 → 段后 2 件
    assert_eq!(render_frame(&ct, 29).items.len(), 1);
    assert_eq!(render_frame(&ct, 45).items.len(), 2);
    assert_eq!(render_frame(&ct, 59).items.len(), 2);
}
