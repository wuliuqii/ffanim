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
}

pub type Result<T> = std::result::Result<T, Error>;
