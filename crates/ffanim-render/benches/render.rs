//! 渲染基准（criterion）：1080p CPU 单帧渲染 + RGB24 打包吞吐。
//!
//! - NF1 验收线：1080p30 全管线实时率 ≥ 1.0x（渲染 + RGB24 打包 ≤ 33ms/帧）；
//! - 数字入库 docs/benchmarks.md（§5.2 性能门禁：随版本记录，>20% 回退必须说明）；
//! - CI 不跑 bench（太慢），`cargo bench -p ffanim-render` 手动/发版前运行。

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};
use ffanim_core::kurbo::{Circle, Rect};
use ffanim_core::{Anim, Color, Item, Timeline};
use ffanim_render::{CpuBackend, RenderBackend};

/// 构造 1080p 满配场景：60 个对象（圆/矩形混合），动画密度贴近教学视频场景。
fn scene_1080p() -> ffanim_core::CompiledTimeline {
    let mut tl = Timeline::new(1920, 1080, 30, Color::new(14, 17, 22, 255));
    for i in 0..60u32 {
        let item = if i % 2 == 0 {
            Item::circle(Circle::new((160.0 + f64::from(i) * 30.0, 540.0), 60.0))
        } else {
            Item::rect(Rect::new(
                100.0 + f64::from(i) * 30.0,
                400.0,
                220.0 + f64::from(i) * 30.0,
                680.0,
            ))
        };
        let id = tl.add(item.with_fill(Color::new(120, 140, 180, 255)));
        tl.play(Anim::fade_in(id).during(1.0));
    }
    tl.compile()
}

/// RGBA8 → RGB24 打包（ffmpeg rawvideo 管道的带宽关键步，R4 瓶颈实测）。
fn pack_rgb24(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len() / 4 * 3);
    for px in rgba.chunks_exact(4) {
        out.extend_from_slice(&px[..3]);
    }
    out
}

/// 1080p 单帧 CPU 渲染（求值 + 光栅化）。
fn bench_cpu_render_1080p(c: &mut Criterion) {
    let tl = scene_1080p();
    let mut backend = CpuBackend::new();
    c.bench_function("cpu_render_1080p", |b| {
        b.iter(|| {
            let draw = ffanim_core::render_frame(&tl, black_box(45));
            backend.render(&draw)
        })
    });
}

/// 1080p 单帧 RGBA→RGB24 打包（≈6MB 源缓冲）。
fn bench_rgb24_pack_1080p(c: &mut Criterion) {
    let tl = scene_1080p();
    let mut backend = CpuBackend::new();
    let rgba = backend.render(&ffanim_core::render_frame(&tl, 45));
    c.bench_function("rgb24_pack_1080p", |b| {
        b.iter(|| pack_rgb24(black_box(&rgba)))
    });
}

criterion_group!(benches, bench_cpu_render_1080p, bench_rgb24_pack_1080p);
criterion_main!(benches);
