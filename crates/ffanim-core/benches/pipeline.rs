//! 编排/求值基准（criterion）：Timeline 构建 + 编译（compile）与帧求值（eval）。
//!
//! 与 ffanim-render 的 render.rs 互补：那边是渲染/打包维度，这边覆盖
//! 「编排 → CompiledTimeline → 查表求值」前半管线。阈值门禁见
//! `scripts/bench_gate.sh`（just bench-gate），数字入库 docs/benchmarks.md。

// criterion_group! 宏展开的注册函数无文档，豁免 missing_docs 门禁（仅本 bench 文件）
#![allow(missing_docs)]

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use ffanim_core::kurbo::{Circle, Rect};
use ffanim_core::{Anim, Color, Item, Timeline};

/// 构造 1080p 满配场景（与 ffanim-render/benches/render.rs 同负载：
/// 60 对象淡入，两处 bench 负载一致才能拼出完整管线数字）。
fn scene_1080p_tl() -> Timeline {
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
    tl
}

/// 编排维度：60 对象 Timeline 构建 + compile（快照调度表生成）。
fn bench_compile_60_items(c: &mut Criterion) {
    c.bench_function("compile_60_items", |b| {
        b.iter(|| scene_1080p_tl().compile())
    });
}

/// 求值维度：CompiledTimeline 单帧查表求值（渲染期纯函数，两阶段模型热路径）。
fn bench_eval_frame_1080p(c: &mut Criterion) {
    let ct = scene_1080p_tl().compile();
    c.bench_function("eval_frame_1080p", |b| {
        b.iter(|| ffanim_core::render_frame(&ct, black_box(45)))
    });
}

criterion_group!(benches, bench_compile_60_items, bench_eval_frame_1080p);
criterion_main!(benches);
