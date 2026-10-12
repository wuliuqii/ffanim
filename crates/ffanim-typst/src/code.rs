//! 代码块排版（P1-13）：Typst raw 块 + syntect 语法高亮。
//!
//! 高亮主题以内嵌 tmTheme（`World::file` 虚拟文件）提供——默认 "Inspired by
//! GitHub" 为浅色语义，深色画布上代码正文近黑不可见，故内置深/浅两套
//! （GitHub Dark/Light 色板）。主题 token 色属排版引擎资产（同 typst-assets
//! 内置字体），不属场景主题色，不走主题 JSON。

/// 代码块高亮主题（内嵌 tmTheme 名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CodeTheme {
    /// 深色主题（GitHub Dark 色板，深色画布默认）。
    Dark,
    /// 浅色主题（GitHub Light 色板，浅色画布用）。
    Light,
}

impl CodeTheme {
    /// World 虚拟文件名（`#set raw(theme: …)` 按此路径经 `World::file` 加载）。
    pub fn file_name(self) -> &'static str {
        match self {
            Self::Dark => "ffanim-dark.tmTheme",
            Self::Light => "ffanim-light.tmTheme",
        }
    }

    /// 普通文本（未命中 scope）的前景色（tmTheme 全局 foreground）。
    ///
    /// Typst raw 高亮只给命中 scope 的 token 设显式 fill，普通文本继承外层
    /// `#set text(fill:)`——故排版源码须显式填入此色（见 `compile_code`）。
    pub fn foreground_hex(self) -> &'static str {
        match self {
            Self::Dark => "#E6EDF3",
            Self::Light => "#1F2328",
        }
    }

    /// 内嵌 tmTheme 内容。
    pub fn tm_theme(self) -> String {
        let (name, fg, comment, string, numeric, keyword, function, type_) = match self {
            Self::Dark => (
                "ffanim-dark",
                "#E6EDF3", // 前景色（覆盖 set text fill）
                "#8B949E", // 注释
                "#A5D6FF", // 字符串
                "#79C0FF", // 数字/常量
                "#FF7B72", // 关键字
                "#D2A8FF", // 函数名
                "#7EE787", // 类型名
            ),
            Self::Light => (
                "ffanim-light",
                "#1F2328",
                "#6A737D",
                "#0A3069",
                "#0550AE",
                "#CF222E",
                "#8250DF",
                "#0550AE",
            ),
        };
        // 精简 tmTheme（plist XML）：foreground 必需，其余按 scope 上色
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>name</key><string>{name}</string>
  <key>settings</key><array>
    <dict><key>settings</key><dict>
      <key>foreground</key><string>{fg}</string>
      <key>background</key><string>#00000000</string>
    </dict></dict>
    <dict><key>scope</key><string>comment</string><key>settings</key><dict><key>foreground</key><string>{comment}</string></dict></dict>
    <dict><key>scope</key><string>string</string><key>settings</key><dict><key>foreground</key><string>{string}</string></dict></dict>
    <dict><key>scope</key><string>constant.numeric</string><key>settings</key><dict><key>foreground</key><string>{numeric}</string></dict></dict>
    <dict><key>scope</key><string>keyword</string><key>settings</key><dict><key>foreground</key><string>{keyword}</string></dict></dict>
    <dict><key>scope</key><string>entity.name.function</string><key>settings</key><dict><key>foreground</key><string>{function}</string></dict></dict>
    <dict><key>scope</key><string>entity.name.type, support.type, storage.type</string><key>settings</key><dict><key>foreground</key><string>{type_}</string></dict></dict>
  </array>
</dict>
</plist>
"#
        )
    }
}

/// 代码块排版选项。
#[derive(Debug, Clone)]
pub struct CodeOptions {
    /// 字号（pt）。
    pub font_size_pt: f64,
    /// 高亮主题。
    pub theme: CodeTheme,
}

impl CodeOptions {
    /// 默认深色主题。
    pub fn new(font_size_pt: f64) -> Self {
        Self {
            font_size_pt,
            theme: CodeTheme::Dark,
        }
    }
}

/// raw 块围栏：取正文中最长反引号串 +1（至少 3），保证不破块。
pub(crate) fn fence_for(body: &str) -> String {
    let max_run = body
        .split(|c: char| c != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat((max_run + 1).max(3))
}

/// 语言标签清洗：只留字母数字与 `#+-_.`，其余截断（防注入排版语法）。
pub(crate) fn sanitize_lang(lang: &str) -> String {
    lang.chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '#' | '+' | '-' | '_' | '.'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 围栏升级() {
        assert_eq!(fence_for("fn main() {}"), "```");
        assert_eq!(fence_for("a ``` b"), "````");
        assert_eq!(fence_for("```` x"), "`````");
    }

    #[test]
    fn 语言清洗() {
        assert_eq!(sanitize_lang("rust"), "rust");
        assert_eq!(sanitize_lang("c++"), "c++");
        assert_eq!(sanitize_lang("c#\nbogus"), "c#");
        assert_eq!(sanitize_lang(""), "");
    }

    #[test]
    fn 主题资产生成() {
        let dark = CodeTheme::Dark.tm_theme();
        assert!(dark.contains("ffanim-dark"));
        assert!(dark.contains("#E6EDF3"));
        // 深/浅两套前景色不同
        assert_ne!(dark, CodeTheme::Light.tm_theme());
    }
}
