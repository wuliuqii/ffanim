//! 错误类型：core 层统一错误。

use std::path::PathBuf;

/// core 层统一错误（`Display` 消息面向终端用户，中文可读）。
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// 主题 JSON 语法或结构不合法。
    #[error("主题文件解析失败: {0}")]
    ThemeParse(#[source] serde_json::Error),

    /// 主题文件路径不存在或无读取权限。
    #[error("主题文件读取失败: {0}")]
    ThemeIo(#[source] std::io::Error, PathBuf),

    /// 主题 JSON 中出现了颜色表未定义的字段名。
    #[error("未知主题颜色字段: {0}")]
    UnknownColorField(String),

    /// 字体文件路径不存在或无读取权限。
    #[error("字体文件读取失败: {1}")]
    FontIo(#[source] std::io::Error, PathBuf),

    /// 字体数据不是合法的 TTF/OTF。
    #[error("字体解析失败（非 TTF/OTF 或数据损坏）")]
    FontParse(#[source] ttf_parser::FaceParsingError),

    /// 主题与命令行均未提供任何可用字体。
    #[error("未找到可用字体，请通过 --font 指定")]
    NoFontFound,

    /// 系统未安装 ffmpeg（编码子进程启动失败）。
    #[error("ffmpeg 未找到，请安装 ffmpeg 或将其加入 PATH")]
    FfmpegNotFound(#[source] std::io::Error),

    /// 编码管线错误（ffmpeg 非零退出或管道中断）。
    #[error("编码失败: {0}")]
    Encode(String),

    /// 未归类错误（保留逃生口）。
    #[error("{0}")]
    Other(String),
}

/// core 层便捷别名：所有 fallible 函数返回此类型。
pub type Result<T> = std::result::Result<T, Error>;
