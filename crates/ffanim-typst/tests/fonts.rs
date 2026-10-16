//! 字体文件显式加载端到端（docs/07 §4.2）：两侧引擎均收 otf/ttf/ttc 路径。
//!
//! 环境探测式：系统无字体目录时打点跳过（CI 容器与 has_ffmpeg 同模式）。
//! 族名命中已由开发期宽度对照验证（注入等宽字体与内嵌衬线宽度显著不同），
//! 正式测试只断言注入行为与缓存正确性，不绑定特定字体文件。

use std::path::{Path, PathBuf};

use ffanim_core::Color;
use ffanim_typst::{TextOptions, TypstEngine};

const FG: Color = Color::new(230, 233, 239, 255);

/// 递归找字体目录下第一个字体文件（深度受限，避免全盘扫描）。
fn find_font(dir: &Path, depth: u8) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    std::fs::read_dir(dir).ok()?.find_map(|e| {
        let p = e.ok()?.path();
        if p.is_dir() {
            find_font(&p, depth - 1)
        } else {
            let hit = matches!(
                p.extension()?.to_str()?,
                "ttf" | "otf" | "ttc" | "TTF" | "OTF" | "TTC"
            );
            hit.then_some(p)
        }
    })
}

/// 系统任一字体文件；无字体环境返回 None（测试打点跳过）。
fn any_font() -> Option<PathBuf> {
    [
        "/usr/share/fonts",
        "/usr/local/share/fonts",
        "/System/Library/Fonts",
        "C:\\Windows\\Fonts",
    ]
    .iter()
    .find_map(|r| find_font(Path::new(r), 4))
}

#[test]
fn typst侧_文件注入可排版且缓存不串() {
    let Some(path) = any_font() else {
        eprintln!("跳过：系统无字体文件");
        return;
    };
    let mut eng = TypstEngine::new(8);
    // add_font：文件注入（ttc 全 face 遍历；fonts_gen 进 LRU 缓存键）
    eng.add_font(&path).expect("字体文件注入应成功");
    // 注入后默认 fallback 链不破坏：内嵌族仍可排版
    let f = eng
        .text("ffanim", &TextOptions::new(24.0, FG))
        .expect("排版应成功");
    assert!(!f.pieces.is_empty() && f.width_pt > 0.0);
    // 同 body 重复排版：LRU 命中且宽度一致（fonts_gen 键不串缓存）
    let again = eng
        .text("ffanim", &TextOptions::new(24.0, FG))
        .expect("复排应成功");
    assert_eq!(again.width_pt, f.width_pt, "同参数复排应逐字节一致");
    // add_font_face：单 face 注入（face 0 对单 face 文件与 ttc 均合法）
    eng.add_font_face(&path, 0).expect("face 0 注入应成功");
    let f2 = eng
        .text("42", &TextOptions::new(24.0, FG))
        .expect("注入 face 后排版应成功");
    assert!(!f2.pieces.is_empty());
}

#[test]
fn 描边侧_文件加载出轮廓() {
    let Some(path) = any_font() else {
        eprintln!("跳过：系统无字体文件");
        return;
    };
    let font = ffanim_core::Font::from_path(&path).expect("描边侧文件加载应成功");
    let outline = font.outline_text("ffanim", 32.0).expect("描边应成功");
    assert!(!outline.elements().is_empty(), "应有路径元素");
}

#[test]
fn 非法face序号报可读错误() {
    let Some(path) = any_font() else {
        eprintln!("跳过：系统无字体文件");
        return;
    };
    let e = ffanim_core::Font::from_index(&path, 9999).expect_err("越界 face 应报错");
    assert!(
        matches!(e, ffanim_core::Error::FontParse(_)),
        "应为字体解析错误: {e}"
    );
}
