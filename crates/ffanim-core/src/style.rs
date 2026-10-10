//! 样式与主题：所有视觉参数数据驱动，从 JSON 加载，代码零硬编码颜色。

use crate::{Error, Result};
use serde::Deserialize;
use std::path::Path;

/// RGBA 颜色，支持 `#RRGGBB` / `#RRGGBBAA` 十六进制解析。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
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
}

/// 描述符：JSON 中以十六进制字符串书写，加载时解析为 [`Color`]。
#[derive(Debug, Clone, Deserialize)]
pub struct ColorSpec(String);

impl ColorSpec {
    pub fn parse(&self) -> Result<Color> {
        Color::from_hex(&self.0)
    }
}

/// 矢量对象默认样式（数据驱动，可被场景级覆盖）。
#[derive(Debug, Clone, Deserialize)]
pub struct Style {
    pub fill: Option<ColorSpec>,
    pub stroke: Option<ColorSpec>,
    pub stroke_width: f32,
}

/// 主题：调色板 + 默认样式 + 公式排版参数。
#[derive(Debug, Clone, Deserialize)]
pub struct Theme {
    pub name: String,
    pub palette: Palette,
    pub style: Style,
    pub formula: FormulaStyle,
}

impl Theme {
    /// 从文件路径加载。
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let json =
            std::fs::read_to_string(path).map_err(|e| Error::ThemeIo(e, path.to_path_buf()))?;
        json.parse::<Self>()
    }

    /// 常用语义色快捷访问（从调色板解析，解析失败属主题文件错误）。
    pub fn background(&self) -> Result<Color> {
        self.palette.background.parse()
    }

    pub fn foreground(&self) -> Result<Color> {
        self.palette.foreground.parse()
    }

    pub fn accent(&self) -> Result<Color> {
        self.palette.accent.parse()
    }
}

/// 从 JSON 字符串解析主题（`"json".parse::<Theme>()`）。
impl std::str::FromStr for Theme {
    type Err = Error;

    fn from_str(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(Error::ThemeParse)
    }
}

/// 调色板。
#[derive(Debug, Clone, Deserialize)]
pub struct Palette {
    pub background: ColorSpec,
    pub foreground: ColorSpec,
    pub accent: ColorSpec,
}

/// 公式排版参数（Typst 层消费）。
#[derive(Debug, Clone, Deserialize)]
pub struct FormulaStyle {
    /// 公式基准字号（pt）。
    pub font_size_pt: f32,
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
        assert_eq!(
            theme.background().unwrap(),
            Color::new(0x0e, 0x11, 0x16, 0xff)
        );
        assert_eq!(theme.accent().unwrap().to_rgba_f32()[3], 1.0);
        assert_eq!(theme.style.stroke_width, 4.0);
        assert_eq!(theme.formula.font_size_pt, 24.0);
    }

    #[test]
    fn 非法颜色报错() {
        assert!(Color::from_hex("0e1116").is_err());
        assert!(Color::from_hex("#zzzzzz").is_err());
        assert!(Color::from_hex("#12345").is_err());
    }
}
