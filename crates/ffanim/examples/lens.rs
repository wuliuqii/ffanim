//! 放大镜透镜（clip 裁剪原语验收示例）：镜片内显示场景矢量级放大内容。
//!
//! 与 pip.rs 的差异：画中画靠「小画布渲染 + 像素 blit」，本例零像素操作——
//! 整帧场景克隆左乘「镜心不动点」放大仿射，包进 `Group` 后 `with_clip(圆)`
//! 一步裁剪，与主帧同一次渲染合成（透镜边缘的错位放大是 clip 的标志性视觉）。
//!
//! 透镜整体（内容组 + 镜框 + 手柄）在出片层组装：位置单一来源 `lens_x(f)`
//! 纯函数，淡入用叶子透明度递归，与引擎时间轴零耦合。
//!
//! 运行：`cargo run -p ffanim --release --example lens`，产物 `out/lens.mp4`。

use ffanim::kurbo::{Affine, Circle, Point, Rect, Shape as _, Vec2};
use ffanim::{Anim, Axes, FrameBackend, Item, Shape, Theme, Timeline, render_frame};
use ffanim_encode::{EncodeOptions, encode_video};
use ffanim_render::RenderBackend;

const W: u32 = 960;
const H: u32 = 540;
const FPS: u32 = 30;

const LENS_R: f64 = 90.0; // 镜片半径
const ZOOM: f64 = 2.5; // 放大倍数
const LENS_X0: f64 = 240.0;
const LENS_X1: f64 = 680.0;
const APPEAR_F: usize = 72; // 透镜淡入起点（2.4s）
const FADE_F: usize = 18; // 淡入时长（0.6s）
const SWEEP_F: usize = 90; // 单程帧数（3.0s，Linear）

fn main() -> ffanim::Result<()> {
    let theme = Theme::from_path(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../themes/default.json"
    ))?;
    let (accent, fg) = (theme.accent(), theme.foreground());
    let mut tl = Timeline::new(W, H, FPS, theme.background());

    // 场景：坐标轴 + 正弦曲线 + 细淡余弦（放大对比更明显）
    let axes = Axes::new((-3.5, 3.5), (-1.5, 1.5))
        .with_size(860.0, 380.0)
        .with_center(460.0, 300.0);
    let ax = tl.add(axes.to_item(&theme.style));
    tl.play(Anim::create(ax).during(0.8));
    let sin = tl.add(axes.plot(|x| x.sin(), (-3.5, 3.5)).with_stroke(fg));
    tl.play(Anim::create(sin).during(1.0));
    let cos = tl.add(
        axes.plot(|x| x.cos(), (-3.5, 3.5))
            .with_stroke(fg.with_alpha(90))
            .with_stroke_width(2.0),
    );
    tl.play(Anim::fade_in(cos).during(0.6));
    tl.wait(7.0); // 透镜淡入 + 两程往返 + 尾停（出片层追加透镜，帧区间见 lens_x）

    let ct = tl.compile();
    let total = ct.duration_frames;
    let out = std::path::Path::new("out/lens.mp4");
    let backend = std::sync::Mutex::new(FrameBackend::auto());
    encode_video(W, H, FPS, total, out, &EncodeOptions::default(), &|f| {
        let mut draw = render_frame(&ct, f);
        let alpha = ((f + FADE_F).saturating_sub(APPEAR_F)).min(FADE_F) as f64 / FADE_F as f64;
        if alpha > 0.0 {
            // 镜心不动点放大：镜心贴 sin 曲线滑过，镜内显示曲线放大段
            // （放大视窗只覆盖镜心周围 R/ZOOM 邻域，镜心必须落在目标上）
            let data_x = (lens_x(f) - 460.0) / 860.0 * 7.0;
            let c = axes.c2p(data_x, data_x.sin());
            let m = Affine::translate(c.to_vec2())
                * Affine::scale(ZOOM)
                * Affine::translate(-c.to_vec2());
            // 镜内 = 背景垫 + 整帧场景克隆（左乘放大仿射），圆 clip 裁出镜片
            let mut inner = vec![
                Item::rect(Rect::new(0.0, 0.0, f64::from(W), f64::from(H)))
                    .with_fill(theme.background())
                    .with_transform(m),
            ];
            inner.extend(draw.items.iter().map(|it| {
                let mut c = it.clone();
                c.transform = m * c.transform;
                c
            }));
            let clip = lens_circle(c);
            let mut lens = Item::group(inner).with_clip(clip);
            apply_opacity(&mut lens, alpha);
            // 镜框 + 手柄（无裁剪，叠加在上）
            let handle_dir = Vec2::new(
                std::f64::consts::FRAC_1_SQRT_2,
                std::f64::consts::FRAC_1_SQRT_2,
            );
            let h0 = c + LENS_R * handle_dir;
            let h1 = h0 + 70.0 * handle_dir;
            let mut deco = Item::group(vec![
                Item::circle(Circle::new(c, LENS_R))
                    .with_stroke(accent)
                    .with_stroke_width(3.5),
                Item::line(h0, h1)
                    .with_stroke(accent)
                    .with_stroke_width(12.0),
            ]);
            apply_opacity(&mut deco, alpha);
            draw.push(lens);
            draw.push(deco);
        }
        backend.lock().unwrap().render(&draw)
    })?;
    println!("OK: {out:?}（{total} 帧，透镜 ×{ZOOM} R={LENS_R}）");
    Ok(())
}

/// 透镜水平位置：前段驻停，之后线性往返（单一来源，边框与内容天然同步）。
fn lens_x(f: usize) -> f64 {
    if f < APPEAR_F + FADE_F {
        return LENS_X0;
    }
    let t = f - APPEAR_F - FADE_F;
    let (seg, u) = (t / SWEEP_F, (t % SWEEP_F) as f64 / SWEEP_F as f64);
    if seg % 2 == 0 {
        LENS_X0 + (LENS_X1 - LENS_X0) * u
    } else {
        LENS_X1 - (LENS_X1 - LENS_X0) * u
    }
}

/// 镜片圆路径（屏幕系，clip 挂在 Group 上作用于整棵子树）。
fn lens_circle(c: Point) -> ffanim::kurbo::BezPath {
    Circle::new(c, LENS_R).to_path(0.1)
}

/// 叶子透明度递归（透镜淡入；与引擎 with_opacity_all 同语义，示例内独立实现）。
fn apply_opacity(item: &mut Item, opacity: f64) {
    item.paint = item.paint.with_opacity(opacity);
    if let Shape::Group(children) = &mut item.shape {
        for c in children {
            apply_opacity(c, opacity);
        }
    }
}
