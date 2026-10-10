//! ffanim-encode：rawvideo 管道 → ffmpeg → segment → concat → mp4。

pub fn placeholder() -> &'static str {
    "ffanim-encode: M2 实现（ffmpeg 探测/自动下载 / 分段并行编码 / concat）"
}
