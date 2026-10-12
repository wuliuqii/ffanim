//! Agent 视觉诊断工具：`strip` 接触表 / `onion` 洋葱皮。
//!
//! 核心洞察（docs/01）：作者无法看视频，把视频变成 PNG/文本/数字。
//! - strip：等距采样 N 帧缩略图拼成网格大图 → 一眼检查全局节奏与构图；
//! - onion：基准帧叠加前后帧半透明残影 → 一眼检查运动轨迹与速度变化。

use std::path::PathBuf;

use ffanim_core::kurbo::Shape as _;
use ffanim_core::{Color, CompiledTimeline, DrawList, Font, Item};
use ffanim_render::{CpuBackend, RenderBackend};

/// strip 输出报告（`--json` 用）。
pub struct StripReport {
    pub out: PathBuf,
    pub frames: Vec<usize>,
    pub cols: usize,
    pub thumb: (u32, u32),
}

/// onion 输出报告（`--json` 用）。
pub struct OnionReport {
    pub out: PathBuf,
    pub frame: usize,
    pub spread: isize,
}

/// 等距采样帧号：覆盖 [0, total-1]，含首尾。
pub fn sample_frames(total: usize, count: usize) -> Vec<usize> {
    if total == 0 {
        return vec![];
    }
    let count = count.max(1).min(total);
    if count == 1 {
        return vec![0];
    }
    (0..count).map(|k| k * (total - 1) / (count - 1)).collect()
}

/// 盒式下采样（预乘面积平均，边缘无暗斑）。
pub fn downsample(src: &[u8], sw: u32, sh: u32, dw: u32, dh: u32) -> Vec<u8> {
    assert!(dw > 0 && dh > 0, "目标尺寸非零");
    let mut out = vec![0u8; (dw * dh * 4) as usize];
    for dy in 0..dh {
        // 目标行对应的源行区间
        let y0 = dy as u64 * sh as u64 / dh as u64;
        let y1 = ((dy + 1) as u64 * sh as u64 / dh as u64).max(y0 + 1);
        for dx in 0..dw {
            let x0 = dx as u64 * sw as u64 / dw as u64;
            let x1 = ((dx + 1) as u64 * sw as u64 / dw as u64).max(x0 + 1);
            // 面积平均：预乘累加（非预乘直接平均会在半透明边缘偏色）
            let (mut r, mut g, mut b, mut a) = (0f64, 0f64, 0f64, 0f64);
            for y in y0..y1 {
                for x in x0..x1 {
                    let o = ((y * sw as u64 + x) * 4) as usize;
                    let (sr, sg, sb, sa) = (
                        f64::from(src[o]),
                        f64::from(src[o + 1]),
                        f64::from(src[o + 2]),
                        f64::from(src[o + 3]),
                    );
                    let w = sa / 255.0;
                    r += sr * w;
                    g += sg * w;
                    b += sb * w;
                    a += sa;
                }
            }
            let n = ((y1 - y0) * (x1 - x0)) as f64;
            let (r, g, b, a) = (r / n, g / n, b / n, a / n);
            // 预乘还原
            let inv = if a <= 0.0 { 0.0 } else { 255.0 / a };
            let o = ((dy * dw + dx) * 4) as usize;
            out[o] = (r * inv).round().min(255.0) as u8;
            out[o + 1] = (g * inv).round().min(255.0) as u8;
            out[o + 2] = (b * inv).round().min(255.0) as u8;
            out[o + 3] = a.round().min(255.0) as u8;
        }
    }
    out
}

/// ghost 叠加：目标不透明（alpha=255），按权重线性混合源帧颜色。
fn blend_ghost(dst: &mut [u8], src: &[u8], alpha: f64) {
    assert_eq!(dst.len(), src.len(), "混合尺寸一致");
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        // 两帧背景均不透明，只需 lerp RGB；A 恒 255
        for k in 0..3 {
            d[k] = (f64::from(s[k]) * alpha + f64::from(d[k]) * (1.0 - alpha)).round() as u8;
        }
    }
}

/// 接触表：等距采样帧 → 缩略网格 PNG。
pub fn strip(
    tl: &CompiledTimeline,
    font: Option<&Font>,
    accent: Color,
    out: PathBuf,
    count: usize,
    cols: usize,
    thumb_w: u32,
) -> Result<StripReport, String> {
    let total = tl.duration_frames.saturating_sub(1).max(1);
    let frames = sample_frames(total, count);
    let cols = cols.max(1).min(frames.len().max(1));
    let rows = frames.len().div_ceil(cols);

    // 缩略尺寸按画布比例
    let (sw, sh) = (tl.width, tl.height);
    let thumb_w = thumb_w.max(64).min(sw);
    let thumb_h = (thumb_w as f64 * sh as f64 / sw as f64).round() as u32;
    let (tw, th) = (thumb_w, thumb_h.max(1));

    // 网格底色：主题背景暗化 50%（数据驱动，不硬编码颜色）
    let bg = tl.background;
    let canvas_bg = Color::new(bg.r / 2, bg.g / 2, bg.b / 2, 255);
    let gap = 4u32;
    let gw = cols as u32 * (tw + gap) + gap;
    let gh = rows as u32 * (th + gap) + gap;
    let mut canvas = vec![0u8; (gw * gh * 4) as usize];
    for px in canvas.chunks_exact_mut(4) {
        px[..4].copy_from_slice(&[canvas_bg.r, canvas_bg.g, canvas_bg.b, 255]);
    }

    let mut backend = CpuBackend::new();
    for (k, &frame) in frames.iter().enumerate() {
        let draw = ffanim_core::render_frame(tl, frame);
        let full = backend.render(&draw);
        let thumb = downsample(&full, sw, sh, tw, th);

        // 帧号标注：透明画布渲染文字轮廓后 over 混合（字体缺失则跳过）
        let thumb = if let Some(font) = font {
            match annotate(font, accent, tw, th, &format!("f{frame}")) {
                Ok(label) => {
                    let mut t = thumb;
                    blend_over(&mut t, &label);
                    t
                }
                Err(e) => {
                    log::warn!("strip 帧号标注跳过: {e}");
                    thumb
                }
            }
        } else {
            thumb
        };

        // 拷入网格
        let (col, row) = (k % cols, k / cols);
        let ox = (gap + col as u32 * (tw + gap)) as usize;
        let oy = (gap + row as u32 * (th + gap)) as usize;
        for y in 0..th as usize {
            let src = &thumb[y * tw as usize * 4..(y + 1) * tw as usize * 4];
            let dst_off = (oy + y) * gw as usize * 4 + ox * 4;
            canvas[dst_off..dst_off + src.len()].copy_from_slice(src);
        }
    }

    ffanim_render::save_png(&out, gw, gh, &canvas)
        .map_err(|e| format!("保存 {out:?} 失败: {e}"))?;
    log::info!(
        "strip 完成: {} 帧采样 → {out:?}（{}x{}，{} 列）",
        frames.len(),
        gw,
        gh,
        cols
    );
    Ok(StripReport {
        out,
        frames,
        cols,
        thumb: (tw, th),
    })
}

/// 渲染帧号标签（透明背景 + accent 文字轮廓）。
fn annotate(font: &Font, accent: Color, w: u32, h: u32, text: &str) -> Result<Vec<u8>, String> {
    let path = font
        .outline_text(text, 20.0)
        .map_err(|e| format!("文字轮廓失败: {e}"))?;
    // 文字轮廓基线在 y=0 且字形向 y 负方向延伸，用 bbox 把左上角对齐到 (4,4)
    let b = path.bounding_box();
    let mut draw = DrawList::new(w, h, Color::new(0, 0, 0, 0));
    draw.push(Item::from_path(path).with_fill(accent).with_transform(
        ffanim_core::kurbo::Affine::translate((4.0 - b.x0, 4.0 - b.y0)),
    ));
    let mut backend = CpuBackend::new();
    Ok(backend.render(&draw))
}

/// 非预乘 over 混合：src 叠加到 dst（同尺寸）。
fn blend_over(dst: &mut [u8], src: &[u8]) {
    assert_eq!(dst.len(), src.len(), "混合尺寸一致");
    for (d, s) in dst.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
        let (sr, sg, sb, sa) = (s[0], s[1], s[2], s[3]);
        if sa == 0 {
            continue;
        }
        let (dr, dg, db, da) = (d[0], d[1], d[2], d[3]);
        let wa = f64::from(sa) / 255.0;
        let wd = f64::from(da) / 255.0 * (1.0 - wa);
        let oa = wa + wd;
        let mix = |sc: u8, dc: u8| -> u8 {
            if oa <= 0.0 {
                0
            } else {
                ((f64::from(sc) * wa + f64::from(dc) * wd) / oa)
                    .round()
                    .min(255.0) as u8
            }
        };
        d[0] = mix(sr, dr);
        d[1] = mix(sg, dg);
        d[2] = mix(sb, db);
        d[3] = ((oa * 255.0).round().min(255.0)) as u8;
    }
}

/// 洋葱皮：基准帧打底，前后 spread 帧按衰减权重叠残影（远→近叠加）。
pub fn onion(
    tl: &CompiledTimeline,
    frame: usize,
    spread: isize,
    out: PathBuf,
) -> Result<OnionReport, String> {
    let last = tl.duration_frames.saturating_sub(1);
    let frame = frame.min(last);
    let spread = spread.clamp(1, 30);
    let (w, h) = (tl.width, tl.height);
    let mut backend = CpuBackend::new();

    // 基准帧打底（不透明）
    let base_draw = ffanim_core::render_frame(tl, frame);
    let mut canvas = backend.render(&base_draw);

    // 偏移从大到小叠加：越靠近基准帧权重越高
    for dist in (1..=spread).rev() {
        for &d in &[-dist, dist] {
            let f = frame as isize + d;
            if !(0..=last as isize).contains(&f) {
                continue;
            }
            let draw = ffanim_core::render_frame(tl, f as usize);
            let ghost = backend.render(&draw);
            let alpha = 0.6 * (1.0 - dist as f64 / (spread + 1) as f64);
            blend_ghost(&mut canvas, &ghost, alpha);
        }
    }

    ffanim_render::save_png(&out, w, h, &canvas).map_err(|e| format!("保存 {out:?} 失败: {e}"))?;
    log::info!("onion 完成: 基准帧 {frame} ±{spread} → {out:?}");
    Ok(OnionReport { out, frame, spread })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 采样帧覆盖首尾() {
        // 均匀覆盖 [0, 99]
        assert_eq!(
            sample_frames(100, 10),
            vec![0, 11, 22, 33, 44, 55, 66, 77, 88, 99]
        );
        // 单帧
        assert_eq!(sample_frames(50, 1), vec![0]);
        // 采样数超过总帧数 → 全帧
        assert_eq!(sample_frames(3, 10), vec![0, 1, 2]);
        // 空时间轴
        assert!(sample_frames(0, 5).is_empty());
    }

    #[test]
    fn 下采样纯色不变与双色平均() {
        // 2x2 纯红 → 1x1 纯红
        let red = vec![
            255u8, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
        ];
        let out = downsample(&red, 2, 2, 1, 1);
        assert_eq!(out, vec![255, 0, 0, 255]);

        // 2x1 左红右蓝 → 1x1 平均（预乘平均后还原：均色紫色 (128,0,128)）
        let rb = vec![255u8, 0, 0, 255, 0, 0, 255, 255];
        let out = downsample(&rb, 2, 1, 1, 1);
        assert_eq!(out[3], 255);
        assert!((i32::from(out[0]) - 128).abs() <= 1, "红色分量: {}", out[0]);
        assert!((i32::from(out[2]) - 128).abs() <= 1, "蓝色分量: {}", out[2]);
    }

    #[test]
    fn ghost混合权重() {
        // alpha=0 → 目标不变；alpha=1 → 完全取源
        let mut dst = vec![100u8, 100, 100, 255];
        blend_ghost(&mut dst, &[200, 200, 200, 255], 0.0);
        assert_eq!(dst[0], 100);
        blend_ghost(&mut dst, &[200, 200, 200, 255], 1.0);
        assert_eq!(dst[0], 200);
        // alpha=0.5 → 150
        blend_ghost(&mut dst, &[100, 100, 100, 255], 0.5);
        assert_eq!(dst[0], 150);
    }

    #[test]
    fn over混合透明源不改目标() {
        let mut dst = vec![10u8, 20, 30, 255];
        blend_over(&mut dst, &[255, 255, 255, 0]);
        assert_eq!(dst, vec![10, 20, 30, 255]);
        // 全不透明白源 → 完全覆盖
        blend_over(&mut dst, &[255, 255, 255, 255]);
        assert_eq!(dst, vec![255, 255, 255, 255]);
    }
}
