//! 音频诊断命令：analyze 报告（时长/电平/削波/静音段）。

use clap::Args;
use std::path::PathBuf;

/// 音频诊断：时长/电平/削波/静音段（agent 排查音轨）
#[derive(Args)]
pub struct AudioArgs {
    /// 音频文件路径
    #[arg(long, short)]
    pub input: PathBuf,

    /// 以 JSON 输出报告
    #[arg(long)]
    pub json: bool,
}

/// 音频诊断报告。
pub fn audio(args: AudioArgs) -> ffanim::Result<()> {
    let AudioArgs { input, json } = args;
    let report = ffanim_audio::analyze(&input)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "input": input,
                "duration": report.duration,
                "sample_rate": report.sample_rate,
                "channels": report.channels,
                "rms_db": report.rms_db,
                "peak_db": report.peak_db,
                "clipped_samples": report.clipped_samples,
                "silence": report.silence,
            })
        );
    } else {
        println!(
            "音频 {input:?}: {:.2}s @{}Hz（{} 声道）",
            report.duration, report.sample_rate, report.channels
        );
        println!(
            "电平: RMS {:.1}dBFS / 峰值 {:.1}dBFS，削波 {} 样本",
            report.rms_db, report.peak_db, report.clipped_samples
        );
        if report.silence.is_empty() {
            println!("静音段: 无");
        } else {
            println!("静音段:");
            for (s, e) in &report.silence {
                println!("  {s:.2}s .. {e:.2}s");
            }
        }
    }
    Ok(())
}
