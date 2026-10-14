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

/// 查找系统中文字体（文件 + face 序号，ttc 集合须取对 face）：环境变量
/// `FFANIM_CJK_FONT`（face 0）→ `fc-list :lang=zh` 带序号查询（常规体、
/// SC 简体、非等宽变体优先）→ fontdb 系统枚举（CoreText/DirectWrite 等
/// 无 fontconfig 平台，与 core 共用同一实现）→ 常见发行版路径扫描（face 0）。
pub fn find_cjk_font() -> Option<(PathBuf, u32)> {
    if let Ok(p) = std::env::var("FFANIM_CJK_FONT") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some((p, 0));
        }
    }
    if let Some(found) = fc_list_cjk() {
        return Some(found);
    }
    if let Some(found) = ffanim_core::find_system_font(CJK_FAMILIES) {
        return Some(found);
    }
    CJK_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|p| p.exists())
        .map(|p| (p, 0))
}

/// fontdb family 匹配用：常见中文字体的 en-US 名（按序优先）。
const CJK_FAMILIES: &[&str] = &[
    "Noto Sans CJK SC",
    "Noto Sans SC",
    "Source Han Sans SC",
    "Source Han Sans CN",
    "PingFang SC",
    "Microsoft YaHei",
];

/// fontconfig 语言查询：列出所有支持中文的 face（文件 + 序号 + 族名），
/// 排序取最优。教训实证（M9.18）：仅按文件名排 bold/italic 会在 Noto CJK
/// ttc 上选中 `NotoSansCJK-Black.ttc`（文件名不含 bold）且其 face 0 是
/// **日文 JP Black**——中文字形用日文写法 + 超粗重量，复/沿等字观感错误。
/// 排序键：常规体（族名/样式无重量后缀）→ SC 简体族 → 非等宽变体（UI/Gothic
/// 的中西文标点排版正确）→ 路径字典序（确定性）。
fn fc_list_cjk() -> Option<(PathBuf, u32)> {
    let out = std::process::Command::new("fc-list")
        .arg("--format")
        .arg("%{file}\t%{index}\t%{family}\t%{style}\n")
        .arg(":lang=zh")
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.trim().splitn(4, '\t');
            let file = it.next()?.trim();
            if file.is_empty() {
                return None;
            }
            let index = it.next()?.trim().parse::<u32>().ok()?;
            let family = it.next().unwrap_or("");
            let style = it.next().unwrap_or("");
            Some((file, index, family, style))
        })
        .min_by_key(|(file, _, family, style)| {
            let lower = format!("{family} {style}").to_lowercase();
            // 重量后缀（Black/Light/Thin/Medium/SemiBold/Bold/Italic…）→ 非常规体
            let weighted = [
                "black", "light", "thin", "medium", "semibold", "bold", "italic", "xlight",
            ]
            .iter()
            .any(|w| lower.contains(w));
            (
                weighted,
                !family.contains("SC"),
                lower.contains("mono") || lower.contains("term") || lower.contains("fixed"),
                file.to_string(),
            )
        })
        .map(|(file, index, _, _)| (PathBuf::from(file), index))
}

/// 常见安装路径兜底（fontconfig 不可用时；Linux/macOS/Windows）。
const CJK_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    "/System/Library/Fonts/PingFang.ttc",
    "C:\\Windows\\Fonts\\msyh.ttc",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_cjk_font_冒烟() {
        // 有中文环境命中路径 + face 序号；无字体环境 None 合法（打点跳过）
        match find_cjk_font() {
            Some((p, idx)) => assert!(p.exists(), "报告的 CJK 字体应存在: {p:?}#{idx}"),
            None => eprintln!("跳过：未发现系统中文字体"),
        }
    }
}
