//! 渲染集成测试：CPU 后端像素级断言（与 mp4 输出同一渲染路径）。
//!
//! 探针法：对已知几何画布取特定像素 RGBA（非预乘、行优先、R 在字节 0），
//! 验证淡入合成、图片平移光栅化与 SVG 渲染。

use ffanim::kurbo::Circle;
use ffanim::{Anim, Color, ImageAsset, Item, Timeline, from_svg_file, from_svg_str, render_frame};
use ffanim_render::{CpuBackend, RenderBackend as _};

const BG: Color = Color::new(14, 17, 22, 255);
const RED: Color = Color::new(255, 0, 0, 255);

fn tl() -> Timeline {
    Timeline::new(96, 54, 30, BG)
}

/// 探针：(x, y) 处像素 RGBA。
fn probe(buf: &[u8], x: usize, y: usize) -> [u8; 4] {
    let i = (y * 96 + x) * 4;
    [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
}

#[test]
fn fade_in_半程像素() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)).with_fill(RED));
    t.play(Anim::fade_in(a).during(1.0));
    let ct = t.compile();

    // 帧 0：进度 0（全透明）→ 无可见对象
    assert!(render_frame(&ct, 0).items.is_empty());
    // 帧 15（0.5s）：alpha = round(255×0.5) = 128 → r ≈ 135，g ≈ 9
    let buf = CpuBackend::new().render(&render_frame(&ct, 15));
    let c = probe(&buf, 48, 27);
    assert!((120..150).contains(&c[0]), "中心应半程红: {c:?}");
    assert!(c[1] < 20, "绿通道应接近背景: {c:?}");
    // 角落（圆外）保持背景
    let corner = probe(&buf, 2, 2);
    assert_eq!(
        [corner[0], corner[1], corner[2]],
        [14, 17, 22],
        "角落应保持背景: {corner:?}"
    );
}

#[test]
fn fade_out_首尾像素() {
    let mut t = tl();
    let a = t.add(Item::circle(Circle::new((48.0, 27.0), 10.0)).with_fill(RED));
    t.play(Anim::fade_out(a).during(1.0));
    let ct = t.compile();

    // 帧 0：淡出未开始 → 全红
    let buf0 = CpuBackend::new().render(&render_frame(&ct, 0));
    assert!(probe(&buf0, 48, 27)[0] > 200, "帧 0 应全红");
    // 帧 29（0.967s）：alpha ≈ 8 → 探针接近背景
    let buf = CpuBackend::new().render(&render_frame(&ct, 29));
    assert!(probe(&buf, 48, 27)[0] < 40, "末帧前应几乎透明");
}

#[test]
fn image_平移像素() {
    // 4x4 不透明红图 → 平移到 (20,10)：探针 (22,12) 红、(0,0) 背景
    let rgba = vec![255u8; 4 * 4 * 4];
    let img = Item::from_image(rgba, 4, 4)
        .expect("合法 RGBA 长度")
        .with_translate(20.0, 10.0);
    let mut t = tl();
    t.add(img);
    let ct = t.compile();

    let buf = CpuBackend::new().render(&render_frame(&ct, 0));
    assert_eq!(probe(&buf, 22, 12)[0], 255, "图内探针应为红");
    assert_eq!(probe(&buf, 0, 0)[0], 14, "画布角落应保持背景");
}

#[test]
fn svg_字符串渲染冒烟() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
        <rect x="0" y="0" width="20" height="20" fill="#ff0000"/>
    </svg>"##;
    let item = from_svg_str(svg).expect("合法 SVG 应可解析");
    let mut t = tl();
    t.add(item);
    let ct = t.compile();

    let draw = render_frame(&ct, 0);
    assert_eq!(draw.items.len(), 1, "SVG 应产出单个对象");
    let buf = CpuBackend::new().render(&draw);
    assert_eq!(probe(&buf, 10, 10)[0], 255, "SVG 矩形内应为红");
}

#[test]
fn svg_文件渲染冒烟() {
    let path = std::env::temp_dir().join(format!("ffanim-render-{}.svg", std::process::id()));
    std::fs::write(
        &path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20">
        <rect x="0" y="0" width="20" height="20" fill="#00ff00"/>
    </svg>"##,
    )
    .expect("写临时 SVG");

    let item = from_svg_file(&path).expect("SVG 文件应可解析");
    let mut t = tl();
    t.add(item);
    let buf = CpuBackend::new().render(&render_frame(&t.compile(), 0));
    assert_eq!(probe(&buf, 10, 10)[1], 255, "SVG 矩形内应为绿");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn image_资产预乘() {
    // 半透明红 (255,0,0,128)：预乘后 rgba = (128,0,0,128)
    let asset = ImageAsset::from_rgba8(vec![255, 0, 0, 128], 1, 1).expect("长度合法");
    assert_eq!(&*asset.rgba, &[128, 0, 0, 128]);
    // 长度不符报错
    assert!(ImageAsset::from_rgba8(vec![0; 3], 1, 1).is_err());
}
