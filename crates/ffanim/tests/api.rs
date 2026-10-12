//! 门面 API 集成测试：re-export 覆盖 + 子系统冒烟（docs/02 规则 B：门面 API 必须被测试引用）。
//!
//! ffmpeg 依赖用例（出片/音频）在无 ffmpeg 环境下打点跳过（CI 容器可能未装）；
//! 其余用例（typst/主题/坐标/诊断纯函数/工具）不依赖外部进程，无条件运行。

use std::path::PathBuf;

use ffanim::kurbo::Circle;
use ffanim::tools::{OnionReport, StripReport, downsample, onion, sample_frames, strip};
use ffanim::typst::{FormulaOptions, TextOptions, TypstEngine, find_cjk_font, match_formulas};
use ffanim::{Anim, Color, Item, Theme, Timeline, VERSION, render_to_mp4};
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
