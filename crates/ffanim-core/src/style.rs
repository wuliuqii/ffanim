//! 样式与主题：所有视觉参数数据驱动，从 JSON 加载，代码零硬编码颜色。

use crate::{Error, Result};
use serde::Deserialize;
use std::path::Path;

/// RGBA 颜色，支持 `#RRGGBB` / `#RRGGBBAA` 十六进制解析。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// 红分量（0-255）。
    pub r: u8,
    /// 绿分量（0-255）。
    pub g: u8,
    /// 蓝分量（0-255）。
    pub b: u8,
    /// alpha 分量（0-255，255 = 不透明）。
    pub a: u8,
}

impl Color {
    /// 常量构造（编译期可用）。
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// 解析 `#RRGGBB` 或 `#RRGGBBAA`。
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex
            .strip_prefix('#')
            .ok_or_else(|| Error::UnknownColorField(format!("颜色必须以 # 开头: {hex}")))?;
        let bytes = hex.as_bytes();
        let pair = |i: usize| -> Result<u8> {
            u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
                .map_err(|_| Error::UnknownColorField(format!("非法十六进制颜色: #{hex}")))
        };
        match bytes.len() {
            6 => Ok(Self::new(pair(0)?, pair(1)?, pair(2)?, 255)),
            8 => Ok(Self::new(pair(0)?, pair(1)?, pair(2)?, pair(3)?)),
            _ => Err(Error::UnknownColorField(format!(
                "颜色长度必须为 6 或 8 位十六进制: #{hex}"
            ))),
        }
    }

    /// 归一化为 [0.0, 1.0] 分量，供渲染层直接使用。
    pub fn to_rgba_f32(self) -> [f32; 4] {
        [
            self.r as f32 / 255.0,
            self.g as f32 / 255.0,
            self.b as f32 / 255.0,
            self.a as f32 / 255.0,
        ]
    }

    /// 颜色线性插值（用于 Transform 形变）。
    pub fn lerp(a: Self, b: Self, t: f64) -> Self {
        let ch = |x: u8, y: u8| {
            ((x as f64 + (y as f64 - x as f64) * t).round() as i64).clamp(0, 255) as u8
        };
        Self::new(ch(a.r, b.r), ch(a.g, b.g), ch(a.b, b.b), ch(a.a, b.a))
    }
}

/// 描述符：JSON 中以十六进制字符串书写，加载时解析为 [`Color`]。
#[derive(Debug, Clone, Deserialize)]
pub struct ColorSpec(String);

impl ColorSpec {
    /// 解析为主题色值。
    pub fn parse(&self) -> Result<Color> {
        Color::from_hex(&self.0)
    }
}

/// 矢量对象默认样式（数据驱动，可被场景级覆盖）。
#[derive(Debug, Clone, Deserialize)]
pub struct Style {
    /// 默认填充色（JSON 十六进制串）。
    pub fill: Option<ColorSpec>,
    /// 默认描边色。
    pub stroke: Option<ColorSpec>,
    /// 默认描边宽度（px）。
    pub stroke_width: f32,
}

/// 主题：调色板 + 默认样式 + 公式排版参数。
///
/// 颜色在加载时一次性解析（解析前置）：`background/foreground/accent` 直接返回
/// [`Color`]，非法十六进制值在 `from_path`/`from_str` 加载点报错，不泄漏到调用点。
#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    /// 主题名（仅展示用）。
    pub name: String,
    /// 调色板（语义色，原始十六进制描述符）。
    pub palette: Palette,
    /// 对象默认样式。
    pub style: Style,
    /// 公式排版参数。
    pub formula: FormulaStyle,
    /// 解析后的语义色缓存（加载入口填充，外部不可见）。
    #[serde(skip)]
    resolved: ResolvedColors,
}

/// 解析后的语义色（仅 [`Theme`] 加载流程写入）。
#[derive(Debug, Clone, Copy)]
struct ResolvedColors {
    background: Color,
    foreground: Color,
    accent: Color,
}

/// serde(skip) 占位：真实值总是由加载入口的 `resolve` 覆盖。
impl Default for ResolvedColors {
    fn default() -> Self {
        Self {
            background: Color::new(0, 0, 0, 255),
            foreground: Color::new(255, 255, 255, 255),
            accent: Color::new(0, 0, 0, 255),
        }
    }
}

impl Theme {
    /// 从文件路径加载（颜色在此处一次性解析，非法值即报错）。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let json =
            std::fs::read_to_string(path).map_err(|e| Error::ThemeIo(e, path.to_path_buf()))?;
        json.parse::<Self>()
    }

    /// 语义色解析前置：非法十六进制在加载点报错（带字段名定位）。
    fn resolve(&mut self) -> Result<()> {
        let parse = |spec: &ColorSpec, field: &str| {
            spec.parse()
                .map_err(|e| Error::UnknownColorField(format!("{field}: {e}")))
        };
        self.resolved = ResolvedColors {
            background: parse(&self.palette.background, "palette.background")?,
            foreground: parse(&self.palette.foreground, "palette.foreground")?,
            accent: parse(&self.palette.accent, "palette.accent")?,
        };
        Ok(())
    }

    /// 画布背景色。
    pub fn background(&self) -> Color {
        self.resolved.background
    }

    /// 前景/文字色。
    pub fn foreground(&self) -> Color {
        self.resolved.foreground
    }

    /// 强调色（高亮、tracker 曲线等）。
    pub fn accent(&self) -> Color {
        self.resolved.accent
    }
}

/// 从 JSON 字符串解析主题（`"json".parse::<Theme>()`）。
impl std::str::FromStr for Theme {
    type Err = Error;

    fn from_str(json: &str) -> Result<Self> {
        let mut theme: Self = serde_json::from_str(json).map_err(Error::ThemeParse)?;
        theme.resolve()?;
        Ok(theme)
    }
}

/// 调色板。
#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    /// 画布背景色。
    pub background: ColorSpec,
    /// 前景/文字色。
    pub foreground: ColorSpec,
    /// 强调色。
    pub accent: ColorSpec,
}

/// 公式排版参数（Typst 层消费）。
#[derive(Debug, Clone, Deserialize)]
pub struct FormulaStyle {
    /// 公式基准字号（pt）。
    pub font_size_pt: f32,
    /// 公式字体族（Typst 字体名，如 "New Computer Modern Math"）。
    pub font_family: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r##"{
        "name": "default",
        "palette": {
            "background": "#0e1116",
            "foreground": "#eceff4",
            "accent": "#88c0d0"
        },
        "style": { "stroke_width": 4.0, "stroke": "#eceff4" },
        "formula": { "font_size_pt": 24.0, "font_family": "NewCM10" }
    }"##;

    #[test]
    fn 加载示例主题() {
        let theme: Theme = SAMPLE.parse().unwrap();
        assert_eq!(theme.name, "default");
        assert_eq!(theme.background(), Color::new(0x0e, 0x11, 0x16, 0xff));
        assert_eq!(theme.accent().to_rgba_f32()[3], 1.0);
        assert_eq!(theme.style.stroke_width, 4.0);
        assert_eq!(theme.formula.font_size_pt, 24.0);
    }

    #[test]
    fn 非法调色板在加载点报错() {
        let bad = SAMPLE.replace("#88c0d0", "#zzzzzz");
        let err = bad.parse::<Theme>().unwrap_err().to_string();
        assert!(err.contains("palette.accent"), "错误应带字段名: {err}");
    }

    #[test]
    fn 非法颜色报错() {
        assert!(Color::from_hex("0e1116").is_err());
        assert!(Color::from_hex("#zzzzzz").is_err());
        assert!(Color::from_hex("#12345").is_err());
    }
}
