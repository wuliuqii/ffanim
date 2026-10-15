//! ffanim-encode：帧序列 → mp4 / gif / webm。
//!
//! 设计（docs/02-架构设计.md §3.5）：
//! - ffmpeg **子进程 + stdin rawvideo 管道**（规避 libav 链接地狱，nf2 零依赖要求）；
//! - 帧段并行：每 worker 独立编码一个 segment（GOP 起点对齐），最后 concat 流复制拼接；
//! - 帧数据由调用方以线程安全闭包供给（渲染是纯函数，多线程无竞争）；
//! - 输出格式按扩展名路由（P1-9）：mp4 = x264 分段直编；webm = VP9 分段直编；
//!   gif = 并行编中转 mp4 → palettegen/paletteuse 两遍调色（`-loop 0` 无限循环）。

use ffanim_core::{Error, Result};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// 进程内编码调用序号（临时目录唯一性用：同进程并发编码各用独立目录）。
static ENCODE_SEQ: AtomicU64 = AtomicU64::new(0);

/// 编码选项。
#[derive(Debug, Clone)]
pub struct EncodeOptions {
    /// 视频编码器（需系统 ffmpeg 支持，如 libx264）。
    pub codec: String,
    /// 质量因子（libx264 CRF，越小质量越高；`quality` 给定时按预设表覆盖）。
    pub crf: u8,
    /// 编码速度/压缩权衡预设。
    pub preset: String,
    /// 并行分段数；None = 自动（核数一半，配合段内 ffmpeg 单线程恰好铺满
    /// 逻辑核；帧数过少时降为 1）。需要全速出片可显式给大值。
    pub segments: Option<usize>,
    /// 画质预设（manim `-ql/-qm/-qh/-qk` 对应）：定输出分辨率缩放 +
    /// 各格式 CRF；None = 原尺寸直出。
    pub quality: Option<Quality>,
}

impl Default for EncodeOptions {
    fn default() -> Self {
        Self {
            codec: "libx264".into(),
            crf: 18,
            preset: "medium".into(),
            segments: None,
            quality: None,
        }
    }
}

/// 画质预设（manim 命令行 `-ql/-qm/-qh/-qk` 对应）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quality {
    /// 低清 854×480 @15fps（草稿预览，出片最快）。
    Low,
    /// 中清 1280×720 @30fps（默认档）。
    Medium,
    /// 高清 1920×1080 @60fps。
    High,
    /// 4K 3840×2160 @60fps。
    Ultra,
}

/// 画质预设参数表（分辨率/fps 定渲染，CRF 分格式定码率）。
#[derive(Debug, Clone, Copy)]
pub struct QualitySpec {
    /// 输出宽度（px，画布缩放目标）。
    pub width: u32,
    /// 输出高度（px）。
    pub height: u32,
    /// 输出帧率（场景时间轴按此 fps 重建，manim 语义：质量定 fps）。
    pub fps: u32,
    /// mp4（libx264）CRF。
    pub crf_x264: u8,
    /// webm（libvpx-vp9）CRF。
    pub crf_vp9: u8,
}

impl Quality {
    /// 解析预设名：`ql/qm/qh/qk`（manim 简写）或 `low/medium/high/4k`，大小写不敏感。
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "ql" | "low" => Ok(Self::Low),
            "qm" | "medium" => Ok(Self::Medium),
            "qh" | "high" => Ok(Self::High),
            "qk" | "4k" | "ultra" => Ok(Self::Ultra),
            other => Err(Error::Encode(format!(
                "未知画质预设 {other:?}（可用: ql/qm/qh/qk 或 low/medium/high/4k）"
            ))),
        }
    }

    /// 预设参数表（manim 质量档位：480p15 / 720p30 / 1080p60 / 2160p60）。
    pub fn spec(self) -> QualitySpec {
        match self {
            Self::Low => QualitySpec {
                width: 854,
                height: 480,
                fps: 15,
                crf_x264: 26,
                crf_vp9: 37,
            },
            Self::Medium => QualitySpec {
                width: 1280,
                height: 720,
                fps: 30,
                crf_x264: 22,
                crf_vp9: 34,
            },
            Self::High => QualitySpec {
                width: 1920,
                height: 1080,
                fps: 60,
                crf_x264: 19,
                crf_vp9: 32,
            },
            Self::Ultra => QualitySpec {
                width: 3840,
                height: 2160,
                fps: 60,
                crf_x264: 16,
                crf_vp9: 29,
            },
        }
    }
}

/// 输出容器格式（按扩展名路由）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    /// mp4（libx264 分段直编，yuv420p）。
    Mp4,
    /// GIF（中转 mp4 → palettegen/paletteuse 两遍调色，`-loop 0` 无限循环）。
    Gif,
    /// WebM（libvpx-vp9 分段直编 + concat 流复制）。
    WebM,
}

/// 按输出路径扩展名识别格式（大小写不敏感）。
fn format_of(out: &Path) -> Result<Format> {
    let ext = out
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("mp4") | Some("mov") | Some("m4v") => Ok(Format::Mp4),
        Some("gif") => Ok(Format::Gif),
        Some("webm") => Ok(Format::WebM),
        other => Err(Error::Encode(format!(
            "无法从扩展名识别输出格式 {other:?}（支持 .mp4 / .gif / .webm）"
        ))),
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

/// 帧序列编码为视频（格式按输出扩展名路由：mp4 / gif / webm）。
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
    let format = format_of(out)?;
    // 输出父目录不存在则创建（脚手架项目首次运行时 out/ 尚不存在）
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::Encode(format!("创建输出目录 {parent:?} 失败: {e}")))?;
    }
    // yuv420p 要求偶数尺寸（画布与预设缩放目标均为偶数）
    assert!(
        width.is_multiple_of(2) && height.is_multiple_of(2),
        "画布尺寸必须为偶数（yuv420p 限制）"
    );

    let segments = opts
        .segments
        .unwrap_or_else(|| {
            // 默认核数一半：每段 = 1 渲染线程 + 1 单线程 ffmpeg，段数×2 恰好
            // 铺满逻辑核不超订阅（全核分段会让笔记本 APU 长时间爆载过热重启）
            std::thread::available_parallelism()
                .map(|n| (n.get() / 2).max(1))
                .unwrap_or(1)
        })
        .max(1)
        .min(total_frames.max(1));

    // 临时目录 = pid + 调用序号：pid 区分进程，序号区分同进程内的并发编码
    //（此前只按 pid 命名，同进程两个并发 encode 共享 seg_XXXX.mp4/concat.txt，
    //  一方结束 remove_dir_all 会删掉另一方正在写的段文件 → concat ENOENT）
    let seq = ENCODE_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!("ffanim-encode-{}-{seq}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| Error::Encode(format!("创建临时目录失败: {e}")))?;
    // 埋点：编码总览（info）
    log::info!(
        "编码开始: {total_frames} 帧 {width}x{height}@{fps} → {out:?}（{format:?}，{segments} 段并行，crf={}）",
        opts.quality.map_or(opts.crf, |q| q.spec().crf_x264)
    );

    let result = match format {
        Format::Gif => {
            // 中转 mp4 不做画质缩放（缩放留在 palette 趟，避免二次重采样）
            let mid_opts = EncodeOptions {
                quality: None,
                ..opts.clone()
            };
            let mid = tmp.join("mid.mp4");
            run_segments(
                &tmp,
                width,
                height,
                fps,
                total_frames,
                &mid,
                &mid_opts,
                frames,
                segments,
                Format::Mp4,
            )
            .and_then(|()| gif_convert(&mid, out, width, height, opts))
        }
        Format::Mp4 | Format::WebM => run_segments(
            &tmp,
            width,
            height,
            fps,
            total_frames,
            out,
            opts,
            frames,
            segments,
            format,
        ),
    };
    let _ = std::fs::remove_dir_all(&tmp);
    result
}

/// 分段编码器的输出参数（编解码器 + 画质预设缩放）。
fn codec_args(fmt: Format, opts: &EncodeOptions) -> Vec<String> {
    let mut args = vec![
        "-pix_fmt".to_string(),
        "yuv420p".to_string(),
        // 每段单线程：并行已由分段承担（段数≈核数/2），段内 x264 帧多线程
        // 会以 1.5×核数/段 叠加（16 段 × 24 线程 ≈ 400 逻辑线程），笔记本
        // APU 长时间全核爆载会触发过热保护重启
        "-threads".to_string(),
        "1".to_string(),
    ];
    match fmt {
        Format::Mp4 => {
            // 画质预设给定时覆盖 opts.crf（manim 语义：质量档定码率）
            let crf = opts.quality.map_or(opts.crf, |q| q.spec().crf_x264);
            args.extend([
                "-c:v".to_string(),
                opts.codec.clone(),
                "-crf".to_string(),
                crf.to_string(),
                "-preset".to_string(),
                opts.preset.clone(),
            ]);
        }
        Format::WebM => {
            let crf = opts.quality.map_or(34, |q| q.spec().crf_vp9);
            args.extend([
                "-c:v".to_string(),
                "libvpx-vp9".to_string(),
                "-crf".to_string(),
                crf.to_string(),
                "-b:v".to_string(),
                "0".to_string(),
                "-row-mt".to_string(),
                "1".to_string(),
                "-cpu-used".to_string(),
                "4".to_string(),
            ]);
        }
        Format::Gif => unreachable!("gif 不走分段直编（先中转 mp4 再调色）"),
    }
    // 画质预设：输出缩放到预设分辨率
    if let Some(q) = opts.quality {
        let s = q.spec();
        args.extend(["-vf".to_string(), format!("scale={}:{}", s.width, s.height)]);
    }
    args
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
    fmt: Format,
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
    let ext = match fmt {
        Format::Mp4 | Format::Gif => "mp4",
        Format::WebM => "webm",
    };
    std::thread::scope(|s| {
        for (i, &(start, end)) in bounds.iter().enumerate() {
            let seg_path: PathBuf = tmp.join(format!("seg_{i:04}.{ext}"));
            let opts = opts.clone();
            s.spawn(move || {
                encode_segment(seg_path, width, height, fps, start, end, &opts, frames, fmt)
            });
        }
    });

    // 单段直接改名，多段 concat 流复制
    if bounds.len() == 1 {
        std::fs::rename(tmp.join(format!("seg_0000.{ext}")), out)
            .map_err(|e| Error::Encode(format!("移动输出失败: {e}")))?;
    } else {
        concat_segments(tmp, out, bounds.len(), ext)?;
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
    fmt: Format,
) -> Result<()> {
    // 埋点：段编码耗时（debug，定位慢段/卡顿帧段用）
    let t0 = std::time::Instant::now();
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
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
    ]);
    // 编解码参数（格式感知：mp4=x264 / webm=vp9）+ 预设缩放
    for a in codec_args(fmt, opts) {
        cmd.arg(a);
    }
    let mut child = match cmd
        .arg(&seg_path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Err(Error::FfmpegNotFound(e)),
    };

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

/// 中转 mp4 → GIF：palettegen/paletteuse 两遍调色 + `-loop 0` 无限循环。
///
/// 画质预设定缩放（None = 源尺寸）；diff 模式调色对动画渐变更友好，
/// bayer 抖动无图案漂移且编码快。
fn gif_convert(
    mid: &Path,
    out: &Path,
    width: u32,
    height: u32,
    opts: &EncodeOptions,
) -> Result<()> {
    let (w, h) = opts.quality.map_or((width, height), |q| {
        let s = q.spec();
        (s.width, s.height)
    });
    let vf = format!(
        "scale={w}:{h}:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];[b][p]paletteuse=dither=bayer:bayer_scale=5"
    );
    let output = Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(mid)
        .args(["-vf", &vf, "-loop", "0", "-an"])
        .arg(out)
        .output()
        .map_err(Error::FfmpegNotFound)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Encode(format!(
            "GIF 调色失败: {}",
            stderr.lines().last().unwrap_or_default()
        )));
    }
    log::info!("GIF 两遍调色完成 → {out:?}（{w}x{h}，-loop 0）");
    Ok(())
}

/// concat demuxer 流复制拼接（分段参数一致时无需重编码）。
fn concat_segments(tmp: &Path, out: &Path, n: usize, ext: &str) -> Result<()> {
    // 埋点：concat 拼接（info）
    log::info!("concat 拼接 {n} 段（流复制，无重编码）");
    let list = tmp.join("concat.txt");
    let body: String = (0..n)
        .map(|i| format!("file 'seg_{i:04}.{ext}'\n"))
        .collect();
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

    /// ffprobe 读取视频流尺寸（ffprobe 缺失/失败返回 None，调用方跳过断言）。
    fn probe_dims(p: &Path) -> Option<(u32, u32)> {
        let out = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=width,height",
                "-of",
                "csv=p=0:s=x",
            ])
            .arg(p)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let s = String::from_utf8_lossy(&out.stdout);
        let mut it = s.trim().split('x');
        Some((it.next()?.parse().ok()?, it.next()?.parse().ok()?))
    }

    /// 合成移动方块帧序列（编码测试共用帧供给）。
    fn moving_square(w: u32, h: u32) -> impl Fn(usize) -> Vec<u8> + Sync {
        let (w, h) = (w as usize, h as usize);
        move |f: usize| -> Vec<u8> {
            let mut buf = vec![14u8; w * h * 4];
            buf.chunks_exact_mut(4).enumerate().for_each(|(px, c)| {
                let (x, y) = ((px % w) as i64, (px / w) as i64);
                let cx = (f * 3) as i64 % w as i64;
                if (x - cx).abs() < 8 && (y - 32).abs() < 8 {
                    c[..].copy_from_slice(&[255, 0, 0, 255]);
                }
            });
            buf
        }
    }

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

    /// 回归：同进程并发编码互不干扰（临时目录须按调用隔离，见 encode_video 内注释）。
    #[test]
    fn 并发编码互不干扰() {
        if check_ffmpeg().is_err() {
            eprintln!("跳过：无 ffmpeg");
            return;
        }
        let (w, h, fps, n) = (64u32, 64u32, 30u32, 60usize);
        let frames = move |f: usize| -> Vec<u8> {
            // 纯色帧，亮度随帧号变化（内容无关紧要，能撑起码流即可）
            let v = (40 + f * 3) as u8;
            vec![v; w as usize * h as usize * 4]
        };

        // 两个线程同时 encode（修复前共享 pid 临时目录，段文件互相覆写/被删）
        let outs: Vec<_> = (0..2)
            .map(|i| std::env::temp_dir().join(format!("ffanim-concurrent-test-{i}.mp4")))
            .collect();
        std::thread::scope(|s| {
            for out in &outs {
                let frames = &frames;
                s.spawn(move || {
                    let opts = EncodeOptions {
                        segments: Some(4),
                        ..Default::default()
                    };
                    encode_video(w, h, fps, n, out, &opts, frames).unwrap();
                });
            }
        });
        for out in &outs {
            let bytes = std::fs::read(out).unwrap();
            assert!(bytes.len() > 1000, "产物过小: {out:?} = {}", bytes.len());
            assert_eq!(&bytes[4..8], b"ftyp");
            let _ = std::fs::remove_file(out);
        }
    }

    /// 画质预设解析：manim 简写/全名大小写不敏感，非法值报可读错误。
    #[test]
    fn quality_解析() {
        assert_eq!(Quality::parse("ql").unwrap(), Quality::Low);
        assert_eq!(Quality::parse("QM").unwrap(), Quality::Medium);
        assert_eq!(Quality::parse("qh").unwrap(), Quality::High);
        assert_eq!(Quality::parse("4k").unwrap(), Quality::Ultra);
        assert_eq!(Quality::parse("low").unwrap(), Quality::Low);
        // 档位参数表对齐 manim：480p15 / 720p30 / 1080p60 / 2160p60
        assert_eq!((854, 480, 15), {
            let s = Quality::Low.spec();
            (s.width, s.height, s.fps)
        });
        assert_eq!((1280, 720, 30), {
            let s = Quality::Medium.spec();
            (s.width, s.height, s.fps)
        });
        assert_eq!((1920, 1080, 60), {
            let s = Quality::High.spec();
            (s.width, s.height, s.fps)
        });
        assert_eq!((3840, 2160, 60), {
            let s = Quality::Ultra.spec();
            (s.width, s.height, s.fps)
        });
        let e = Quality::parse("qq").unwrap_err();
        assert!(e.to_string().contains("qq"));
    }

    /// GIF 输出：GIF8 魔数 + NETSCAPE2.0 循环扩展（-loop 0 无限循环）。
    #[test]
    fn gif_循环输出() {
        if check_ffmpeg().is_err() {
            eprintln!("跳过：无 ffmpeg");
            return;
        }
        let (w, h, fps, n) = (64u32, 64u32, 30u32, 30usize);
        let out = std::env::temp_dir().join("ffanim-encode-test.gif");
        let opts = EncodeOptions {
            segments: Some(2), // 多段走 concat 中转路径
            quality: Some(Quality::Low),
            ..Default::default()
        };
        encode_video(w, h, fps, n, &out, &opts, &moving_square(w, h)).unwrap();

        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(&bytes[..6], b"GIF89a", "应为合法 GIF 容器");
        assert!(
            bytes.windows(11).any(|w| w == b"NETSCAPE2.0"),
            "应有 NETSCAPE2.0 循环扩展（-loop 0）"
        );
        let _ = std::fs::remove_file(&out);
    }

    /// WebM 输出：EBML 魔数（VP9 分段直编 + concat 流复制）。
    #[test]
    fn webm_vp9输出() {
        if check_ffmpeg().is_err() {
            eprintln!("跳过：无 ffmpeg");
            return;
        }
        let (w, h, fps, n) = (64u32, 64u32, 30u32, 30usize);
        let out = std::env::temp_dir().join("ffanim-encode-test.webm");
        let opts = EncodeOptions {
            segments: Some(2),
            ..Default::default()
        };
        encode_video(w, h, fps, n, &out, &opts, &moving_square(w, h)).unwrap();

        let bytes = std::fs::read(&out).unwrap();
        assert_eq!(
            &bytes[..4],
            &[0x1A, 0x45, 0xDF, 0xA3],
            "应为 EBML/WebM 容器"
        );
        let _ = std::fs::remove_file(&out);
    }

    /// 画质预设缩放生效：mp4 输出流尺寸 = 预设分辨率（ffprobe 实证）。
    #[test]
    fn quality_预设缩放生效() {
        if check_ffmpeg().is_err() {
            eprintln!("跳过：无 ffmpeg");
            return;
        }
        let (w, h, fps, n) = (960u32, 540u32, 30u32, 10usize);
        let out = std::env::temp_dir().join("ffanim-encode-quality-test.mp4");
        let opts = EncodeOptions {
            quality: Some(Quality::Low),
            ..Default::default()
        };
        encode_video(w, h, fps, n, &out, &opts, &moving_square(w, h)).unwrap();

        let Some((pw, ph)) = probe_dims(&out) else {
            eprintln!("跳过：ffprobe 不可用");
            return;
        };
        assert_eq!((pw, ph), (854, 480), "输出应缩放到 ql 预设分辨率");
        let _ = std::fs::remove_file(&out);
    }

    /// 未知扩展名报可读错误。
    #[test]
    fn 未知扩展名报错() {
        let out = std::env::temp_dir().join("ffanim-encode-test.avi");
        let e = encode_video(64, 64, 30, 2, &out, &Default::default(), &|_| {
            vec![0; 64 * 64 * 4]
        })
        .unwrap_err();
        assert!(e.to_string().contains("扩展名"), "错误应可读: {e}");
    }
}
