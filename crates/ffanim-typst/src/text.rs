//! 富文本选项与系统中文字体查找（Text 走 Typst markup 管线，复用 Formula 产物）。
//!
//! 排版产物与公式同为 [`crate::Formula`]（pieces + 尺寸），`to_items` /
//! `fit_transform` 原样复用；区别仅在源码模式（markup 而非 math）与
//! 换行/行距选项。

use std::path::PathBuf;

use ffanim_core::Color;

/// 文本排版选项。
#[derive(Debug, Clone)]
pub struct TextOptions {
    /// 字号（pt；Formula 内部坐标同为 pt，px 摆放由 `fit_transform` 的 scale 定）。
    pub font_size_pt: f64,
    /// 文字填充色（数据驱动：来自主题）。
    pub text_color: Color,
    /// 段落内容宽度（pt），超过自动换行；None = 不限宽（单行）。
    pub wrap_width_pt: Option<f64>,
    /// 行距（em 倍数，Typst `par.leading`；0.65 与 Typst 默认一致）。
    pub leading_em: f64,
}

impl TextOptions {
    /// 单行文本选项（不限宽，默认行距）。
    pub fn new(font_size_pt: f64, text_color: Color) -> Self {
        Self {
            font_size_pt,
            text_color,
            wrap_width_pt: None,
            leading_em: 0.65,
        }
    }
}

/// 查找系统中文字体文件（首个存在者）：环境变量 `FFANIM_CJK_FONT`
/// → `fc-list :lang=zh`（fontconfig 按语言能力查询）→ 常见发行版路径扫描。
pub fn find_cjk_font() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("FFANIM_CJK_FONT") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }
    if let Some(p) = fc_list_cjk() {
        return Some(p);
    }
    CJK_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
}

/// fontconfig 语言查询：列出所有支持中文的字体文件，取常规体优先的第一个。
fn fc_list_cjk() -> Option<PathBuf> {
    let out = std::process::Command::new("fc-list")
        .arg("--format")
        .arg("%{file}\n")
        .arg(":lang=zh")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        // 常规体优先（Bold/Italic 样式排后），保证正文观感
        .min_by_key(|l| {
            let lower = l.to_lowercase();
            (
                lower.contains("bold") || lower.contains("italic"),
                l.to_string(),
            )
        })
        .map(PathBuf::from)
}

/// 常见安装路径兜底（fontconfig 不可用时；Linux/macOS/Windows）。
const CJK_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "C:\\Windows\\Fonts\\msyh.ttc",
];
