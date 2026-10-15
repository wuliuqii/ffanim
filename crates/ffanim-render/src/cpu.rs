//! CPU 兜底后端：vello_cpu（sparse strips），无 GPU 依赖，CI 与无 GPU 环境使用。

use crate::{RenderBackend, render_cpu_item, walk_items};
use ffanim_core::DrawList;

/// CPU 渲染后端。
#[derive(Debug, Default, Clone, Copy)]
pub struct CpuBackend;

impl CpuBackend {
    /// 创建 CPU 后端（无状态，可按帧独立实例化以支持分段并行）。
    pub fn new() -> Self {
        Self
    }
}

impl RenderBackend for CpuBackend {
    fn render(&mut self, draw: &DrawList) -> Vec<u8> {
        // 埋点：逐帧渲染入口（trace，排查丢帧/空白帧用）
        log::trace!(
            "CpuBackend::render: {}x{}，{} 项",
            draw.width,
            draw.height,
            draw.items.len()
        );
        // vello_cpu 尺寸上限为 u16
        assert!(draw.width <= u16::MAX as u32 && draw.height <= u16::MAX as u32);
        let (w, h) = (draw.width as u16, draw.height as u16);
        let mut ctx = vello_cpu::RenderContext::new(w, h);

        // 背景：整屏矩形填充
        ctx.set_paint(crate::to_peniko(draw.background));
        ctx.fill_rect(&vello_cpu::kurbo::Rect::new(
            0.0,
            0.0,
            f64::from(w),
            f64::from(h),
        ));

        // 场景对象：深度优先遍历，Group 的子项继承累积变换
        walk_items(
            &draw.items,
            vello_cpu::kurbo::Affine::IDENTITY,
            &mut |t, item| {
                render_cpu_item(&mut ctx, crate::to_kurbo(t), item);
            },
        );

        ctx.flush();
        let mut pixmap = vello_cpu::Pixmap::new(w, h);
        ctx.render(&mut pixmap, &mut vello_cpu::Resources::new());

        // vello_cpu 输出预乘 RGBA8 → 还原为非预乘（PNG 标准）
        pixmap
            .data()
            .iter()
            .flat_map(|px| {
                let (r, g, b, a) = (px.r, px.g, px.b, px.a);
                unpremultiply(r, a)
                    .into_iter()
                    .chain(unpremultiply(g, a))
                    .chain(unpremultiply(b, a))
                    .chain([a])
            })
            .collect()
    }
}

/// 预乘分量还原：c = c' / a。
#[inline]
fn unpremultiply(c: u8, a: u8) -> [u8; 1] {
    if a == 0 || a == 255 {
        [c]
    } else {
        [((u16::from(c) * 255 + u16::from(a) / 2) / u16::from(a)) as u8]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffanim_core::{Color, DrawList, Item};
    use kurbo::{Affine, Circle};

    #[test]
    fn 背景与图形像素() {
        let bg = Color::new(14, 17, 22, 255);
        let mut draw = DrawList::new(64, 64, bg);
        // 中心一个填充红圆
        draw.push(
            Item::circle(Circle::new((32.0, 32.0), 16.0)).with_fill(Color::new(255, 0, 0, 255)),
        );

        let mut backend = CpuBackend;
        let buf = backend.render(&draw);
        assert_eq!(buf.len(), (64 * 64 * 4) as usize);

        // 角落 = 背景
        let corner = &buf[0..4];
        assert_eq!(corner, &[bg.r, bg.g, bg.b, bg.a]);

        // 中心 = 红（近似，抗锯齿中心点应为纯红）
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(
            buf[center] > 200,
            "中心应为红色: {:?}",
            &buf[center..center + 4]
        );

        // 半径外应为背景
        let edge = ((32 * 64 + 2) * 4) as usize;
        assert_eq!(&buf[edge..edge + 4], &[bg.r, bg.g, bg.b, bg.a]);
    }

    #[test]
    fn 组合变换继承() {
        let mut draw = DrawList::new(64, 64, Color::new(0, 0, 0, 255));
        // 组合整体右移 32px，内部圆在 (0,0)
        draw.push(Item {
            id: 1,
            shape: ffanim_core::Shape::Group(vec![
                Item::circle(Circle::new((0.0, 0.0), 8.0)).with_fill(Color::new(0, 255, 0, 255)),
            ]),
            paint: Default::default(),
            transform: Affine::translate((32.0, 32.0)),
        });

        let mut backend = CpuBackend;
        let buf = backend.render(&draw);
        let center = ((32 * 64 + 32) * 4) as usize;
        assert!(
            buf[center + 1] > 200,
            "变换后中心应为绿色: {:?}",
            &buf[center..center + 4]
        );
    }

    #[test]
    fn 组内alpha描边线渲染() {
        let mut draw = DrawList::new(64, 64, Color::new(0, 0, 0, 255));
        // group 内半透明细线（circle_sine 引导线同款样式）
        draw.push(Item {
            id: 1,
            shape: ffanim_core::Shape::Group(vec![
                Item::line((8.0, 32.0), (56.0, 32.0))
                    .with_stroke(Color::new(255, 255, 255, 120))
                    .with_stroke_width(1.5),
            ]),
            paint: Default::default(),
            transform: Affine::IDENTITY,
        });

        let mut backend = CpuBackend;
        let buf = backend.render(&draw);
        // 线上多点应高于背景（alpha 120 细线混合后 ≈ 100+）
        for x in [16usize, 32, 48] {
            let px = (32 * 64 + x) * 4;
            assert!(
                buf[px] > 60,
                "x={x} 处应有半透明线像素: {:?}",
                &buf[px..px + 4]
            );
        }
    }

    #[test]
    fn 布尔挖洞形状渲染() {
        use ffanim_core::ops::BoolOp;
        // (8,8)-(56,56) 差掉 (24,24)-(40,40)：环形填充，洞应透出背景
        let a = ffanim_core::Item::rect(kurbo::Rect::new(8.0, 8.0, 56.0, 56.0))
            .with_fill(Color::new(255, 0, 0, 255));
        let b = ffanim_core::Item::rect(kurbo::Rect::new(24.0, 24.0, 40.0, 40.0));
        let ring = a.boolean(BoolOp::Difference, &b).unwrap();

        let mut draw = DrawList::new(64, 64, Color::new(0, 0, 0, 255));
        draw.push(ring);
        let buf = CpuBackend.render(&draw);

        fn px(buf: &[u8], x: usize, y: usize) -> u8 {
            buf[(y * 64 + x) * 4]
        }
        assert_eq!(px(&buf, 16, 32), 255, "环体应为红");
        assert_eq!(px(&buf, 32, 32), 0, "洞中心应为背景");
    }

    #[test]
    fn 图片渲染冒烟() {
        // 2x2 纯红图片平移到 (3,3)，画布 8x8
        let mut rgba = vec![0u8; 4 * 4];
        for px in rgba.chunks_exact_mut(4) {
            px.copy_from_slice(&[255, 0, 0, 255]);
        }
        let mut draw = DrawList::new(8, 8, Color::new(0, 0, 0, 255));
        draw.push(
            Item::from_image(rgba, 2, 2)
                .unwrap()
                .with_translate(3.0, 3.0),
        );
        let buf = CpuBackend.render(&draw);
        // (4,4) 在图片内 = 红
        let idx = ((4 * 8 + 4) * 4) as usize;
        assert_eq!(
            &buf[idx..idx + 4],
            &[255, 0, 0, 255],
            "{:?}",
            &buf[idx..idx + 4]
        );
        // 角落 = 背景
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn 图片透明度渲染() {
        // 2x2 纯红图片 opacity 0.5 叠在黑底上 → 中心 ≈ 128 红
        let mut rgba = vec![0u8; 4 * 4];
        for px in rgba.chunks_exact_mut(4) {
            px.copy_from_slice(&[255, 0, 0, 255]);
        }
        let mut item = Item::from_image(rgba, 2, 2).unwrap();
        item.paint.opacity = 0.5;
        let mut draw = DrawList::new(8, 8, Color::new(0, 0, 0, 255));
        draw.push(item.with_translate(3.0, 3.0));
        let buf = CpuBackend.render(&draw);
        let idx = ((4 * 8 + 4) * 4) as usize;
        assert!(
            (100..=160).contains(&buf[idx]),
            "透明度 0.5 中心应为半红: {:?}",
            &buf[idx..idx + 4]
        );
    }
}
