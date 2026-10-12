//! ffanim-audio：音频诊断（analyze）、`Duration::FromAudio` 时长推导、音轨 mux。
//!
//! 设计：解码与混流复用 ffmpeg 子进程（与 ffanim-encode 同一路径，不引新依赖），
//! 分析逻辑纯 Rust 实现（RMS/峰值/削波/静音段）。
//! 注：电平为 RMS dBFS 近似，非 ITU-R BS.1770 LUFS（K 加权留待需要时引入）。

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use ffanim_core::{Error, Result};

/// 解码统一目标格式：单声道 48kHz f32。
const DECODE_SR: u32 = 48_000;

/// 音频诊断报告（`audio analyze` / `--json` 用）。
#[derive(Debug, Clone)]
pub struct AudioReport {
    /// 源文件路径。
    pub path: PathBuf,
    /// 时长（秒）
    pub duration: f64,
    /// 采样率（Hz）。
    pub sample_rate: u32,
    /// 声道数。
    pub channels: usize,
    /// 全程 RMS 电平（dBFS，0 = 满幅正弦）
    pub rms_db: f64,
    /// 采样峰值（dBFS）
    pub peak_db: f64,
    /// 削波样本数（|x| ≥ 0.999）
    pub clipped_samples: usize,
    /// 静音段（起点秒, 终点秒）：窗口电平低于 -60dB 持续 ≥ 0.5s
    pub silence: Vec<(f64, f64)>,
}

/// PCM 统计（纯函数，可测）。
#[derive(Debug, Clone, PartialEq)]
pub struct PcmStats {
    /// 全程 RMS 电平（dBFS）。
    pub rms_db: f64,
    /// 峰值电平（dBFS）。
    pub peak_db: f64,
    /// 削波样本数（|x| ≥ 1.0）。
    pub clipped_samples: usize,
    /// 静音区间（起止秒）。
    pub silence: Vec<(f64, f64)>,
}

/// 分析 PCM 样本（假定 [-1,1] 归一化）。
///
/// - RMS/峰值：全样本统计；
/// - 静音段：50ms 窗 RMS < -60dB，连续 ≥ 0.5s 合并为段。
pub fn analyze_pcm(samples: &[f32], sample_rate: u32) -> PcmStats {
    assert!(sample_rate > 0);
    let mut sum_sq = 0f64;
    let mut peak = 0f64;
    let mut clipped = 0usize;
    for &s in samples {
        let x = f64::from(s);
        sum_sq += x * x;
        peak = peak.max(x.abs());
        if x.abs() >= 0.999 {
            clipped += 1;
        }
    }
    let rms = if samples.is_empty() {
        0.0
    } else {
        (sum_sq / samples.len() as f64).sqrt()
    };
    let to_db = |v: f64| {
        if v <= f64::EPSILON {
            -f64::INFINITY
        } else {
            20.0 * v.log10()
        }
    };

    // 静音窗检测：窗长 50ms，窗 RMS 阈值 -60dB
    const WIN_MS: usize = 50;
    const QUIET_DB: f64 = -60.0;
    const MIN_SILENCE_S: f64 = 0.5;
    let win = (sample_rate as usize * WIN_MS / 1000).max(1);
    let mut silence = Vec::new();
    let mut quiet_start: Option<usize> = None;
    for (wi, chunk) in samples.chunks(win).enumerate() {
        let w_rms = (chunk
            .iter()
            .map(|s| f64::from(*s) * f64::from(*s))
            .sum::<f64>()
            / chunk.len().max(1) as f64)
            .sqrt();
        let start = wi * win;
        if to_db(w_rms) < QUIET_DB {
            if quiet_start.is_none() {
                quiet_start = Some(start);
            }
        } else if let Some(s) = quiet_start.take() {
            push_silence(&mut silence, s, start, sample_rate, MIN_SILENCE_S);
        }
    }
    if let Some(s) = quiet_start.take() {
        push_silence(&mut silence, s, samples.len(), sample_rate, MIN_SILENCE_S);
    }

    PcmStats {
        rms_db: to_db(rms),
        peak_db: to_db(peak),
        clipped_samples: clipped,
        silence,
    }
}

/// 静音段达到最小长度才记录（段尾为 exclusive 采样下标 → 秒）。
fn push_silence(out: &mut Vec<(f64, f64)>, start: usize, end: usize, sr: u32, min_s: f64) {
    let dur = (end - start) as f64 / f64::from(sr);
    if dur >= min_s {
        out.push((start as f64 / f64::from(sr), end as f64 / f64::from(sr)));
    }
}

/// 分析音频文件：ffmpeg 解码为单声道 f32 PCM 后统计。
pub fn analyze(path: &Path) -> Result<AudioReport> {
    let samples = decode_pcm(path)?;
    let stats = analyze_pcm(&samples, DECODE_SR);
    let report = AudioReport {
        path: path.to_path_buf(),
        duration: samples.len() as f64 / f64::from(DECODE_SR),
        sample_rate: DECODE_SR,
        channels: 1,
        rms_db: stats.rms_db,
        peak_db: stats.peak_db,
        clipped_samples: stats.clipped_samples,
        silence: stats.silence,
    };
    log::info!(
        "audio analyze: {path:?}（{:.2}s，RMS {:.1}dBFS，峰值 {:.1}dBFS，削波 {}，静音 {} 段）",
        report.duration,
        report.rms_db,
        report.peak_db,
        report.clipped_samples,
        report.silence.len()
    );
    Ok(report)
}

/// 解码任意输入为单声道 48kHz f32le PCM（经 stdout 管道）。
fn decode_pcm(path: &Path) -> Result<Vec<f32>> {
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(path)
        .args([
            "-f",
            "f32le",
            "-ac",
            "1",
            "-ar",
            &DECODE_SR.to_string(),
            "-",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(Error::FfmpegNotFound)?;

    let mut buf = Vec::new();
    child
        .stdout
        .take()
        .expect("stdout 已 piped")
        .read_to_end(&mut buf)
        .map_err(Error::FfmpegNotFound)?;
    let status = child.wait().map_err(Error::FfmpegNotFound)?;
    if !status.success() {
        return Err(Error::Encode(format!("ffmpeg 解码音频失败: {path:?}")));
    }
    Ok(buf
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect())
}

/// 音轨混流：视频流复制 + 音频转 AAC（`Duration::FromAudio` 出片的最后一步）。
pub fn mux(video: &Path, audio: &Path, out: &Path) -> Result<()> {
    let output = Command::new("ffmpeg")
        .args(["-y", "-v", "error", "-i"])
        .arg(video)
        .args(["-i"])
        .arg(audio)
        .args([
            "-map", "0:v", "-map", "1:a", "-c:v", "copy", "-c:a", "aac", "-b:a", "192k",
        ])
        .arg(out)
        .output()
        .map_err(Error::FfmpegNotFound)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Encode(format!(
            "音轨混流失败: {}",
            stderr.lines().last().unwrap_or_default()
        )));
    }
    log::info!("音轨混流完成: {video:?} + {audio:?} → {out:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 440Hz 正弦（幅度 0.8）1s：RMS ≈ -4.9dB，无削波，无静音段。
    #[test]
    fn 正弦统计() {
        let sr = 48_000u32;
        let samples: Vec<f32> = (0..sr)
            .map(|i| {
                (0.8 * (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sr as f64).sin()) as f32
            })
            .collect();
        let stats = analyze_pcm(&samples, sr);
        assert!(
            (stats.rms_db - (-4.90)).abs() < 0.1,
            "正弦 RMS 应 ≈ -4.90dBFS: {}",
            stats.rms_db
        );
        assert!(
            (stats.peak_db - (-1.94)).abs() < 0.01,
            "峰值: {}",
            stats.peak_db
        );
        assert_eq!(stats.clipped_samples, 0);
        assert!(stats.silence.is_empty());
    }

    /// 满幅方波应检测到削波。
    #[test]
    fn 削波检测() {
        let samples = vec![1.0f32; 1000];
        let stats = analyze_pcm(&samples, 48_000);
        assert_eq!(stats.clipped_samples, 1000);
        assert!(stats.peak_db.abs() < 0.01);
    }

    /// 中段插入 1s 静音 → 检出静音段（0.5, 1.5）。
    #[test]
    fn 静音段检测() {
        let sr = 48_000u32;
        let tone: Vec<f32> = (0..sr)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sr as f64).sin() as f32)
            .collect();
        let mut samples = tone.clone();
        samples.extend(std::iter::repeat_n(0.0, sr as usize)); // 1s 静音
        samples.extend(tone);
        let stats = analyze_pcm(&samples, sr);
        assert_eq!(
            stats.silence.len(),
            1,
            "应检出 1 段静音: {:?}",
            stats.silence
        );
        let (s, e) = stats.silence[0];
        assert!((s - 0.95).abs() < 0.1, "静音起点: {s}");
        assert!((e - 2.05).abs() < 0.1, "静音终点: {e}");
    }

    /// 短于 0.5s 的静音不记录。
    #[test]
    fn 短静音忽略() {
        let sr = 48_000u32;
        let tone: Vec<f32> = (0..sr)
            .map(|i| (2.0 * std::f64::consts::PI * 440.0 * i as f64 / sr as f64).sin() as f32)
            .collect();
        let mut samples = tone.clone();
        samples.extend(std::iter::repeat_n(0.0, sr as usize * 3 / 10)); // 0.3s 静音
        samples.extend(tone);
        let stats = analyze_pcm(&samples, sr);
        assert!(
            stats.silence.is_empty(),
            "0.3s 静音不应记录: {:?}",
            stats.silence
        );
    }
}
