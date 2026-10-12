//! 时间轴语义集成测试：游标、overlap、调度、相机、缓动（docs/02 §3.4 规则 B 覆盖）。
//!
//! 调度类断言查 `CompiledTimeline::duration_frames`（构建期游标语义），
//! 求值类断言查 `render_frame` 产物（段中插值/段后基线/透明度传导），
//! 相机与缓动用 CPU 后端做像素级实证（与 mp4 输出同一渲染路径）。

use ffanim::kurbo::{Affine, Circle};
use ffanim::{Anim, AnimKind, Camera, Color, DrawList, Item, RateFunc, Timeline, render_frame};
use ffanim_render::{CpuBackend, RenderBackend as _};

const BG: Color = Color::new(14, 17, 22, 255);
const RED: Color = Color::new(255, 0, 0, 255);

fn tl() -> Timeline {
    Timeline::new(96, 54, 30, BG)
}

#[test]
fn play_游标累加() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)));
    t.play(Anim::create(a).during(1.0));
    t.play(Anim::fade_in(a).during(1.0));
    // 两段各 1.0s @30fps：游标累加到 2.0s = 60 帧
    assert_eq!(t.compile().duration_frames, 60);
}

#[test]
fn overlap_回退游标() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)));
    t.play(Anim::fade_in(a).during(1.0));
    t.overlap(0.5);
    t.play(Anim::fade_out(a).during(1.0));
    // 第二段从 0.5s 开始，总长 1.5s = 45 帧
    assert_eq!(t.compile().duration_frames, 45);
}

#[test]
fn wait_追加空白() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)));
    t.play(Anim::fade_in(a).during(1.0));
    t.wait(0.5);
    assert_eq!(t.compile().duration_frames, 45);
}

#[test]
fn group_固定步长调度() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((30.0, 27.0), 8.0)));
    let b = t.add(Item::circle(Circle::new((66.0, 27.0), 8.0)));
    t.group(
        [Anim::fade_in(a).during(1.0), Anim::fade_in(b).during(1.0)],
        0.25,
    );
    // 第二段起点 0.25s，总长 1.25s = 37.5 → 38 帧
    let ct = t.compile();
    assert_eq!(ct.duration_frames, 38);
    // 帧 0：两段进度均为 0（全透明）→ 无可见对象
    assert!(render_frame(&ct, 0).items.is_empty());
    // 帧 7（0.233s）：a 已淡入中、b 未开始（0.25s 起）→ 仅 a
    assert_eq!(render_frame(&ct, 7).items.len(), 1);
    // 帧 30（1.0s）：a 已完成、b 进度 0.75 → 两项
    assert_eq!(render_frame(&ct, 30).items.len(), 2);
}

#[test]
fn lagged_start_间隔语义() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((30.0, 27.0), 8.0)));
    let b = t.add(Item::circle(Circle::new((66.0, 27.0), 8.0)));
    t.lagged_start(
        [Anim::fade_in(a).during(1.0), Anim::fade_in(b).during(1.0)],
        0.25,
    );
    // 间隔 = ratio × 前段时长 = 0.25s，与 group(…, 0.25) 调度等价
    assert_eq!(t.compile().duration_frames, 38);
}

#[test]
fn fade_in_半程透明度() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)).with_fill(RED));
    t.play(Anim::fade_in(a).during(1.0));
    let ct = t.compile();
    // 帧 15（0.5s）：进度 0.5 → alpha = round(255×0.5) = 128
    let draw = render_frame(&ct, 15);
    assert_eq!(draw.items.len(), 1);
    let paint = &draw.items[0].paint;
    assert!(
        (paint.opacity - 0.5).abs() < 1e-9,
        "opacity = {}",
        paint.opacity
    );
    let fill = paint.fill.expect("淡入中的对象应有填充色");
    assert_eq!(fill.a, 128, "填充 alpha = {}", fill.a);
}

#[test]
fn transform_to_段中插值与段后基线() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((20.0, 27.0), 10.0)));
    // 构建期推演检查：TransformTo 记录 id/from/to，from 取当前基线（恒等）
    let anim = Anim::transform_to(a, Affine::translate((80.0, 0.0))).during(1.0);
    assert!(
        matches!(anim.kind, AnimKind::TransformTo { .. }),
        "应为 TransformTo: {:?}",
        anim.kind
    );
    t.play(anim);
    let ct = t.compile();
    // 段中帧 15：DrawList 携带插值后的对象（DrawList 为门面公开项）
    let draw: DrawList = render_frame(&ct, 15);
    let tx_mid = draw.items[0].transform.as_coeffs()[4];
    assert!((tx_mid - 40.0).abs() < 1e-9, "段中 tx = {tx_mid}");
    // 段后取存储基线（= 目标）：越界帧与编译期基线均为 to
    let tx_after = render_frame(&ct, 45).items[0].transform.as_coeffs()[4];
    assert!((tx_after - 80.0).abs() < 1e-9, "段后 tx = {tx_after}");
    let tx_base = ct.items[a as usize].transform.as_coeffs()[4];
    assert!((tx_base - 80.0).abs() < 1e-9, "基线 tx = {tx_base}");
}

#[test]
fn camera_推近像素() {
    // zoom 2.0：探针 (63,27) 距屏心 15px；zoom 1 时对应世界距 15（圆 r=10 外，背景），
    // zoom 2 时对应世界距 7.5（圆内，红）——像素实证 world→screen 仿射
    let mk = |zoom: f64| {
        let mut t = tl();
        t.camera(Camera {
            center: (48.0, 27.0).into(),
            zoom,
            rotation: 0.0,
        });
        let c = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)).with_fill(RED));
        t.play(Anim::fade_in(c).during(1.0));
        t.compile()
    };
    let mut cpu = CpuBackend::new();
    let base = cpu.render(&render_frame(&mk(1.0), 29));
    let zoomed = cpu.render(&render_frame(&mk(2.0), 29));
    let red = |buf: &[u8]| buf[(27 * 96 + 63) * 4];
    assert!(red(&base) < 50, "无缩放探针应为背景: r={}", red(&base));
    assert!(
        red(&zoomed) > 200,
        "zoom 2 后探针应在圆内: r={}",
        red(&zoomed)
    );
}

#[test]
fn rate_func_去而复返() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((20.0, 27.0), 8.0)).with_translate(20.0, 0.0));
    t.play(
        Anim::transform_to(a, Affine::translate((100.0, 0.0)))
            .rate(RateFunc::ThereAndBack)
            .during(2.0),
    );
    let ct = t.compile();
    // 中点帧 30（t=0.5）：ThereAndBack 进度恰为 1 → 到达目标 tx=100
    let tx_mid = render_frame(&ct, 30).items[0].transform.as_coeffs()[4];
    assert!((tx_mid - 100.0).abs() < 1e-9, "中点 tx = {tx_mid}");
    // 末帧 59（t≈0.983）：回程带 Smooth 缓动，进度回落到 ≈0.003 → tx ≈ 20.26（贴回起点）
    let tx_end = render_frame(&ct, 59).items[0].transform.as_coeffs()[4];
    assert!(
        (20.0..25.0).contains(&tx_end),
        "末帧应回到起点附近: tx = {tx_end}"
    );
}
