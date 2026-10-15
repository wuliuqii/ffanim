//! 画中画（manim `ZoomedScene` 对齐）**纯出片层拼装**验证示例。
//!
//! 结论先行：零新增 API、零核心改动——出片层自由组合三个现有公开件：
//! 1. [`ffanim::render_frame`] 取主帧 `DrawList`；
//! 2. 克隆一份，所有顶层对象左乘「源中心 → 窗口中心」放大仿射，用
//!    [`CpuBackend`] 渲染成窗口尺寸小帧（**画布边界天然裁剪**，替代 clip 层）；
//! 3. blit 回主帧 + 像素描边框，交给 [`encode_video`] 编码。
//!
//! 放大是矢量级重渲染（非像素重采样），线宽随缩放变粗——与 manim 缩放
//! mobject 的视觉一致。动点走 `Linear` rate，源区中心恒定，拼装端与引擎
//! 缓动零耦合。
//!
//! 运行：`cargo run -p ffanim --release --example pip`，产物 `out/pip.mp4`。

use ffanim::kurbo::{Affine, Circle, Point, Rect};
use ffanim::{Anim, Axes, Item, RateFunc, Theme, Timeline, render_frame};
use ffanim_encode::{EncodeOptions, encode_video};
use ffanim_render::{CpuBackend, RenderBackend};

const W: u32 = 960;
const H: u32 = 540;
const FPS: u32 = 30;

// 放大镜：窗口 320x240 挂右上角，源区 160x120（= 窗口 / 放大倍数）
const PIP_W: u32 = 320;
const PIP_H: u32 = 240;
const PIP_X: u32 = 628;
const PIP_Y: u32 = 16;
const ZOOM: f64 = 2.0;

fn main() -> ffanim::Result<()> {
    let theme = Theme::from_path(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../themes/default.json"
    ))?;
    let accent = theme.accent();
    let fg = theme.foreground();
    let mut tl = Timeline::new(W, H, FPS, theme.background());

    // 主场景：坐标轴 + 正弦曲线，中心略偏左下给放大镜让位
    let axes = Axes::new((-3.5, 3.5), (-1.5, 1.5))
        .with_size(860.0, 380.0)
        .with_center(460.0, 300.0);
    let ax = tl.add(axes.to_item(&theme.style));
    tl.play(Anim::create(ax).during(0.8));

    // 源区指示框：告诉观众放大镜在看主视图哪里（源中心 = c2p(0,0) = 轴心）
    let src_c = axes.c2p(0.0, 0.0);
    let half = (PIP_W as f64 / ZOOM / 2.0, PIP_H as f64 / ZOOM / 2.0);
    let indicator = tl.add(
        Item::rect(Rect::new(
            src_c.x - half.0,
            src_c.y - half.1,
            src_c.x + half.0,
            src_c.y + half.1,
        ))
        .with_stroke(accent.with_alpha(140))
        .with_stroke_width(1.5),
    );
    tl.play(Anim::fade_in(indicator).during(0.4));

    let sine = tl.add(axes.plot(|x| x.sin(), (-3.5, 3.5)).with_stroke(fg));
    tl.play(Anim::create(sine).during(1.0));

    // 动点：tracker 驱动每帧重建（点 + 到 x 轴的垂线）
    let x = tl.add_tracker(-3.0);
    let dot = tl.add_updater(&[x], move |v| {
        let p = axes.c2p(v[0], v[0].sin());
        let foot = axes.c2p(v[0], 0.0);
        Item::group(vec![
            Item::line(foot, p)
                .with_stroke(fg.with_alpha(120))
                .with_stroke_width(1.5),
            Item::circle(Circle::new(p, 8.0)).with_fill(accent),
        ])
    });
    tl.play(Anim::fade_in(dot).during(0.3));
    tl.wait(0.3);

    // 动点往返横穿源区：Linear rate → 拼装端帧→参数精确换算（与引擎零耦合）
    tl.play(Anim::set_value(x, 3.0).during(3.5).rate(RateFunc::Linear));
    tl.play(Anim::set_value(x, -3.0).during(3.5).rate(RateFunc::Linear));
    tl.wait(0.5);

    // —— 出片层画中画管线：每帧 = 主帧 + 矢量放大小帧 blit + 边框 ——
    let ct = tl.compile();
    let total = ct.duration_frames;
    let out = std::path::Path::new("out/pip.mp4");
    encode_video(W, H, FPS, total, out, &EncodeOptions::default(), &|f| {
        pip_frame(&ct, f, src_c, fg)
    })?;
    println!("OK: {out:?}（{total} 帧，画中画 {PIP_W}x{PIP_H} ×{ZOOM}）");
    Ok(())
}

/// 单帧合成：主帧渲染 → 放大重渲染小帧 → blit → 描边框。
fn pip_frame(ct: &ffanim::CompiledTimeline, f: usize, src_c: Point, fg: ffanim::Color) -> Vec<u8> {
    let draw = render_frame(ct, f);
    let mut buf = CpuBackend::new().render(&draw);

    // 矢量放大：源中心 → 窗口中心的仿射（小帧画布边界天然裁剪出窗内容）
    let m = Affine::translate((
        f64::from(PIP_W) / 2.0 - ZOOM * src_c.x,
        f64::from(PIP_H) / 2.0 - ZOOM * src_c.y,
    )) * Affine::scale(ZOOM);
    let mut pip = draw;
    // 小帧用窗口尺寸渲染（否则克隆 960x540 canvas，blit 行距错位撕裂）
    pip.width = PIP_W;
    pip.height = PIP_H;
    for it in &mut pip.items {
        it.transform = m * it.transform;
    }
    let pb = CpuBackend::new().render(&pip);

    // 小帧 blit 到主帧窗口区（同为非预乘 RGBA8，逐行拷贝）
    let (pw, ph) = (PIP_W as usize, PIP_H as usize);
    let stride = W as usize * 4;
    for row in 0..ph {
        let s = row * pw * 4;
        let d = (PIP_Y as usize + row) * stride + PIP_X as usize * 4;
        buf[d..d + pw * 4].copy_from_slice(&pb[s..s + pw * 4]);
    }

    // 窗口外沿 2px 边框
    stroke_rect(&mut buf, W as usize, H as usize, 2, fg);
    buf
}

/// 主帧缓冲上给放大镜窗口（`PIP_X/PIP_Y/PIP_W/PIP_H`）描边框（外沿 `t` 像素厚）。
fn stroke_rect(buf: &mut [u8], w: usize, h: usize, t: usize, c: ffanim::Color) {
    let (x0, y0, rw, rh) = (
        PIP_X as usize,
        PIP_Y as usize,
        PIP_W as usize,
        PIP_H as usize,
    );
    let (r, g, b) = (c.r, c.g, c.b);
    let put = |buf: &mut [u8], x: usize, y: usize| {
        if x < w && y < h {
            let i = (y * w + x) * 4;
            buf[i..i + 3].copy_from_slice(&[r, g, b]);
        }
    };
    for d in 0..t {
        for xx in x0 - d..=x0 + rw + d - 1 {
            put(buf, xx, y0 - 1 - d);
            put(buf, xx, y0 + rh + d);
        }
        for yy in y0..y0 + rh {
            put(buf, x0 - 1 - d, yy);
            put(buf, x0 + rw + d, yy);
        }
    }
}
