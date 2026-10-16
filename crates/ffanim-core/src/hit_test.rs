//! 命中测试（docs/07 §5.1）：屏幕点 → 命中 Item。
//!
//! 纯几何层（零渲染依赖）：从顶层（z 序最高）向下遍历，Item 逆变换到局部
//! 坐标后判定——fill 面走非零环绕（kurbo `Shape::winding`），描边带走段级
//! 最近距离（`ParamCurveNearest`）；Group 递归子树（组与子件可同时命中），
//! clip 路径须包含命中点，透明（alpha=0）不命中。上层交互工具（点击选中/
//! 悬停提示）由此起步，本层不持有任何渲染器或时间轴状态。

use kurbo::ParamCurveNearest as _;
use kurbo::Shape as _;
use kurbo::{Affine, BezPath, Point};

use crate::item::{Item, ItemId, Paint, Shape};

/// 命中测试：返回命中对象 id 列表，**顶层（z 序最高）在前**。
///
/// `slack` 为描边判定额外容差（像素），供粗指针/小目标放大命中区。
/// 同一帧内嵌套结构（Group 与其子件、被重叠的多个对象）可同时返回，
/// 按绘制语义从最上层到最下层排列。
pub fn hit_test(items: &[Item], pt: Point, slack: f64) -> Vec<ItemId> {
    let mut ids = Vec::new();
    for item in items.iter().rev() {
        hit_item(item, Affine::IDENTITY, pt, slack, &mut ids);
    }
    ids
}

/// 首个命中（最顶层对象）；无命中返回 `None`。
pub fn hit_test_first(items: &[Item], pt: Point, slack: f64) -> Option<ItemId> {
    hit_test(items, pt, slack).into_iter().next()
}

/// 递归判定单个对象：`world` 为父链累计变换（世界 → 父局部）。
fn hit_item(item: &Item, world: Affine, pt: Point, slack: f64, out: &mut Vec<ItemId>) {
    // 世界 → 本 Item 局部（kurbo 0.13 inverse 直接返回 Affine：退化矩阵
    // 产生 inf/NaN 系数，判废为不可命中）
    let inv = (world * item.transform).inverse();
    if !inv.as_coeffs().iter().all(|v| v.is_finite()) {
        return;
    }
    let local = inv * pt;
    // clip 门：命中点须落在本 Item 局部坐标的裁剪路径内
    if let Some(clip) = &item.clip
        && clip.winding(local) == 0
    {
        return;
    }
    match &item.shape {
        Shape::Path(p) => {
            if path_hit(p, &item.paint, local, slack) {
                out.push(item.id);
            }
        }
        Shape::Group(children) => {
            // 组是否命中 = 任一子件命中；子件（更具体）排在组 id 之前
            let before = out.len();
            for child in children.iter().rev() {
                hit_item(child, world * item.transform, pt, slack, out);
            }
            if out.len() > before {
                out.push(item.id);
            }
        }
        Shape::Image(img) => {
            // 图片命中 = 局部点落位图矩形内（透明度系数为 0 视为不可见）
            if item.paint.opacity > 0.0
                && local.x >= 0.0
                && local.y >= 0.0
                && local.x <= f64::from(img.width)
                && local.y <= f64::from(img.height)
            {
                out.push(item.id);
            }
        }
    }
}

/// 路径命中：fill 面非零环绕优先，其次描边带（半宽 + slack）。
fn path_hit(p: &BezPath, paint: &Paint, pt: Point, slack: f64) -> bool {
    if paint.fill.is_some_and(|c| c.a > 0) && p.winding(pt) != 0 {
        return true;
    }
    if paint.stroke.is_some_and(|c| c.a > 0) && paint.stroke_width > 0.0 {
        let r_sq = (paint.stroke_width * 0.5 + slack).powi(2);
        return p
            .segments()
            .any(|seg| seg.nearest(pt, 0.1).distance_sq <= r_sq);
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::Item;
    use crate::style::Color;

    const RED: Color = Color::new(220, 80, 80, 255);
    const CLEAR: Color = Color::new(220, 80, 80, 0);

    fn fill_rect(x0: f64, y0: f64, x1: f64, y1: f64, fill: Color) -> Item {
        Item::rect(kurbo::Rect::new(x0, y0, x1, y1)).with_fill(fill)
    }

    #[test]
    fn 矩形填充面命中与脱靶() {
        let a = fill_rect(10.0, 10.0, 60.0, 50.0, RED).with_id(1);
        let items = vec![a];
        assert_eq!(hit_test_first(&items, Point::new(30.0, 30.0), 0.0), Some(1));
        assert_eq!(hit_test_first(&items, Point::new(70.0, 30.0), 0.0), None);
        // fill 无 slack 概念：边缘外 0.5px 仍脱靶
        assert_eq!(hit_test_first(&items, Point::new(60.5, 30.0), 0.0), None);
    }

    #[test]
    fn 描边带命中含容差() {
        let mut line = Item::line((0.0, 0.0), (100.0, 0.0)).with_id(1);
        line.paint.stroke = Some(RED);
        line.paint.stroke_width = 4.0;
        let items = vec![line];
        // 线上（fill 为 None，仅描边带判定）
        assert_eq!(hit_test_first(&items, Point::new(50.0, 0.0), 0.0), Some(1));
        // 线旁 2px < 半宽 2 + slack 0 → 命中边界内
        assert_eq!(hit_test_first(&items, Point::new(50.0, 1.9), 0.0), Some(1));
        // 线旁 3px 超出半宽，slack=0 脱靶 / slack=2 命中
        assert_eq!(hit_test_first(&items, Point::new(50.0, 3.0), 0.0), None);
        assert_eq!(hit_test_first(&items, Point::new(50.0, 3.0), 2.0), Some(1));
    }

    #[test]
    fn 透明对象不命中() {
        let a = fill_rect(0.0, 0.0, 10.0, 10.0, CLEAR).with_id(1);
        let items = vec![a];
        assert_eq!(hit_test_first(&items, Point::new(5.0, 5.0), 0.0), None);
    }

    #[test]
    fn 变换后命中跟随() {
        // 矩形整体平移 (100, 50)：屏幕点按世界坐标判
        let a = fill_rect(0.0, 0.0, 20.0, 20.0, RED)
            .with_id(1)
            .with_transform(kurbo::Affine::translate((100.0, 50.0)));
        let items = vec![a];
        assert_eq!(
            hit_test_first(&items, Point::new(110.0, 60.0), 0.0),
            Some(1)
        );
        assert_eq!(hit_test_first(&items, Point::new(10.0, 60.0), 0.0), None);
    }

    #[test]
    fn 组递归且顶层优先() {
        // 底层大矩形 + 顶层小矩形：重叠点两件全命中、小件在前
        let bottom = fill_rect(0.0, 0.0, 100.0, 100.0, RED).with_id(1);
        let top = fill_rect(40.0, 40.0, 60.0, 60.0, RED).with_id(2);
        let group = Item::group(vec![bottom, top]).with_id(7);
        let items = vec![group];
        let hits = hit_test(&items, Point::new(50.0, 50.0), 0.0);
        assert_eq!(hits, vec![2, 1, 7], "两子件全命中（顶在前），组 id 收尾");
        // 只压在大矩形上的点：子件 + 组
        let hits2 = hit_test(&items, Point::new(10.0, 10.0), 0.0);
        assert_eq!(hits2, vec![1, 7]);
        // 组外点：全脱靶
        assert!(hit_test(&items, Point::new(200.0, 200.0), 0.0).is_empty());
    }

    #[test]
    fn 裁剪路径排除命中() {
        let clip_only_left = {
            let mut c = BezPath::new();
            c.move_to((0.0, 0.0));
            c.line_to((50.0, 0.0));
            c.line_to((50.0, 100.0));
            c.line_to((0.0, 100.0));
            c.close_path();
            c
        };
        let a = fill_rect(0.0, 0.0, 100.0, 100.0, RED)
            .with_id(1)
            .with_clip(clip_only_left);
        let items = vec![a];
        assert_eq!(hit_test_first(&items, Point::new(25.0, 50.0), 0.0), Some(1));
        assert_eq!(hit_test_first(&items, Point::new(75.0, 50.0), 0.0), None);
    }

    #[test]
    fn 图片对象矩形命中() {
        let img = Item::from_image(vec![255; 4 * 4], 2, 2)
            .expect("小图构造")
            .with_id(1);
        let items = vec![img];
        assert_eq!(hit_test_first(&items, Point::new(1.0, 1.0), 0.0), Some(1));
        assert_eq!(hit_test_first(&items, Point::new(3.0, 1.0), 0.0), None);
    }

    #[test]
    fn 多对象重叠返回全序() {
        let a = fill_rect(0.0, 0.0, 50.0, 50.0, RED).with_id(1);
        let b = fill_rect(20.0, 20.0, 80.0, 80.0, RED).with_id(2);
        let c = fill_rect(40.0, 40.0, 90.0, 90.0, RED).with_id(3);
        let items = vec![a, b, c];
        // 三件重叠点：c(最上) → b → a
        assert_eq!(hit_test(&items, Point::new(45.0, 45.0), 0.0), vec![3, 2, 1]);
        // 仅 b 覆盖点：b 在前（此时 a 不在 b 覆盖区内）
        assert_eq!(hit_test(&items, Point::new(25.0, 25.0), 0.0), vec![2, 1]);
    }
}
