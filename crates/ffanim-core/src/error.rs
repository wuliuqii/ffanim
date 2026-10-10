//! 错误类型：core 层统一错误。

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("主题文件解析失败: {0}")]
    ThemeParse(#[source] serde_json::Error),

    #[error("主题文件读取失败: {0}")]
    ThemeIo(#[source] std::io::Error, PathBuf),

    #[error("未知主题颜色字段: {0}")]
    UnknownColorField(String),

    #[error("字体文件读取失败: {1}")]
    FontIo(#[source] std::io::Error, PathBuf),

    #[error("字体解析失败（非 TTF/OTF 或数据损坏）")]
    FontParse(#[source] ttf_parser::FaceParsingError),

    #[error("未找到可用字体，请通过 --font 指定")]
    NoFontFound,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
