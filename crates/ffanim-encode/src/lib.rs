//! ffanim-encode：帧序列 → mp4。
//!
//! 设计（docs/02-架构设计.md §3.5）：
//! - ffmpeg **子进程 + stdin rawvideo 管道**（规避 libav 链接地狱，nf2 零依赖要求）；
//! - 帧段并行：每 worker 独立编码一个 segment（GOP 起点对齐），最后 concat 流复制拼接；
//! - 帧数据由调用方以线程安全闭包供给（渲染是纯函数，多线程无竞争）。

use ffanim_core::{Error, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// 编码选项。
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// 视频编码器（需系统 ffmpeg 支持，如 libx264）。
    pub codec: String,
    /// 质量因子（libx264 CRF，越小质量越高）。
    pub crf: u8,
    /// 编码速度/压缩权衡预设。
    pub preset: String,
    /// 并行分段数；None = 自动（CPU 核数，帧数过少时降为 1）。
    pub segments: Option<usize>,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            codec: "libx264".into(),
            crf: 18,
            preset: "medium".into(),
            segments: None,
        }
    }
}

/// 校验 ffmpeg 可用性（找不到时给出可行动的错误信息）。
pub fn check_ffmpeg() -> Result<()> {
    Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|_| ())
        .map_err(Error::FfmpegNotFound)
}

/// 帧序列编码为视频。
///
/// `frames` 为线程安全的取帧函数：frame_idx → 非预乘 RGBA8 字节。
pub fn encode_video(
    width: u32,
    height: u32,
    fps: u32,
    total_frames: usize,
    out: &Path,
    opts: &EncodeOptions,
    frames: &(impl Fn(usize) -> Vec<u8> + Sync),
) -> Result<()> {
    check_ffmpeg()?;
    // 输出父目录不存在则创建（脚手架项目首次运行时 out/ 尚不存在）
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Encode(format!("创建输出目录 {parent:?} 失败: {e}")))?;
    }
    // yuv420p 要求偶数尺寸
    assert!(
        width.is_multiple_of(2) && height.is_multiple_of(2),
        "画布尺寸必须为偶数（yuv420p 限制）"
    );

    let segments = opts
        .segments
        .unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1)
        })
        .max(1)
        .min(total_frames.max(1));

    let tmp = std::env::temp_dir().join(format!("ffanim-encode-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| Error::Encode(format!("创建临时目录失败: {e}")))?;
    // 埋点：编码总览（info）
    log::info!(
        "编码开始: {total_frames} 帧 {width}x{height}@{fps} → {out:?}（{segments} 段并行，{} crf={}）",
        opts.codec,
        opts.crf
    );

    let result = run_segments(
        &tmp,
        width,
        height,
        fps,
        total_frames,
        out,
        opts,
        frames,
        segments,
    );
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

#[allow(clippy::too_many_arguments)]
fn run_segments(
    tmp: &Path,
    width: u32,
    height: u32,
    fps: u32,
    total_frames: usize,
    out: &Path,
    opts: &EncodeOptions,
    frames: &(impl Fn(usize) -> Vec<u8> + Sync),
    segments: usize,
) -> Result<()> {
    // 帧段划分
    let chunk = total_frames.div_ceil(segments);
    let bounds: Vec<(usize, usize)> = (0..segments)
        .map(|s| {
            let start = s * chunk;
            (start, (start + chunk).min(total_frames))
        })
        .filter(|(s, e)| e > s)
        .collect();
    // 埋点：帧段划分（debug）
    log::debug!("帧段划分: {bounds:?}");

    // 每段一个 worker（scoped threads：借用 frames，无需 'static）
    std::thread::scope(|s| {
        for (i, &(start, end)) in bounds.iter().enumerate() {
            let tmp: PathBuf = tmp.join(format!("seg_{i:04}.mp4"));
            let opts = opts.clone();
            s.spawn(move || encode_segment(tmp, width, height, fps, start, end, &opts, frames));
        }
    });

    // 单段直接改名，多段 concat 流复制
    if bounds.len() == 1 {
        std::fs::rename(tmp.join("seg_0000.mp4"), out)
            .map_err(|e| Error::Encode(format!("移动输出失败: {e}")))?;
    } else {
        concat_segments(tmp, out, bounds.len())?;
    }
    Ok(())
}

/// 编码一个连续帧段。
#[allow(clippy::too_many_arguments)]
fn encode_segment(
    seg_path: PathBuf,
    width: u32,
    height: u32,
    fps: u32,
    start: usize,
    end: usize,
    opts: &EncodeOptions,
    frames: &(impl Fn(usize) -> Vec<u8> + Sync),
) -> Result<()> {
    // 埋点：段编码耗时（debug，定位慢段/卡顿帧段用）
    let t0 = std::time::Instant::now();
    let mut child = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "-s",
            &format!("{width}x{height}"),
            "-r",
            &fps.to_string(),
            "-i",
            "pipe:0",
            "-pix_fmt",
            "yuv420p",
            "-c:v",
            &opts.codec,
            "-crf",
            &opts.crf.to_string(),
            "-preset",
            &opts.preset,
        ])
        .arg(&seg_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(Error::FfmpegNotFound)?;

    let mut stdin = child.stdin.take().expect("stdin 已接管");
    for f in start..end {
        let rgba = frames(f);
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
        stdin
            .write_all(&rgba)
            .map_err(|e| Error::Encode(format!("帧数据写入失败: {e}")))?;
    }
    drop(stdin); // 关闭 stdin 触发 ffmpeg flush

    let output = child
        .wait_with_output()
        .map_err(|e| Error::Encode(e.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Encode(format!(
            "ffmpeg 段编码失败（{}..{end} 帧）: {}",
            start,
            stderr.lines().last().unwrap_or_default()
        )));
    }
    log::debug!(
        "段 {start}..{end} 编码完成（{:.2}s）",
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

/// concat demuxer 流复制拼接（分段参数一致时无需重编码）。
fn concat_segments(tmp: &Path, out: &Path, n: usize) -> Result<()> {
    // 埋点：concat 拼接（info）
    log::info!("concat 拼接 {n} 段（流复制，无重编码）");
    let list = tmp.join("concat.txt");
    let body: String = (0..n).map(|i| format!("file 'seg_{i:04}.mp4'\n")).collect();
    std::fs::write(&list, body).map_err(|e| Error::Encode(e.to_string()))?;

    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "concat",
            "-safe",
            "0",
            "-i",
        ])
        .arg(&list)
        .args(["-c", "copy"])
        .arg(out)
        .output()
        .map_err(Error::FfmpegNotFound)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Encode(format!(
            "concat 失败: {}",
            stderr.lines().last().unwrap_or_default()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合成渐变帧，10 段并行编码 120 帧，验证 mp4 产物。
    #[test]
    fn 编码最小视频() {
        if check_ffmpeg().is_err() {
            eprintln!("跳过：无 ffmpeg");
            return;
        }
        let (w, h, fps, n) = (64u32, 64u32, 30u32, 120usize);
        // 每帧一个移动的红色方块
        let frames = |f: usize| -> Vec<u8> {
            let mut buf = vec![14u8; w as usize * h as usize * 4];
            buf.chunks_exact_mut(4).enumerate().for_each(|(px, c)| {
                let (x, y) = ((px % w as usize) as i64, (px / w as usize) as i64);
                let cx = (f * 3) as i64 % w as i64;
                if (x - cx).abs() < 8 && (y - 32).abs() < 8 {
                    c[..].copy_from_slice(&[255, 0, 0, 255]);
                }
            });
            buf
        };

        let out = std::env::temp_dir().join("ffanim-encode-test.mp4");
        let opts = EncodeOptions {
            segments: Some(4),
            ..Default::default()
        };
        encode_video(w, h, fps, n, &out, &opts, &frames).unwrap();

        let bytes = std::fs::read(&out).unwrap();
        assert!(bytes.len() > 1000, "产物过小: {}", bytes.len());
        assert_eq!(&bytes[4..8], b"ftyp", "应为合法 mp4 容器");
        let _ = std::fs::remove_file(&out);
    }
}
