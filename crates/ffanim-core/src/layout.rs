//! 相对布局（manim `next_to` / `to_edge` / `arrange` 对应）：构建期纯操作。
//!
//! 全部基于 [`Item::bbox`]（含变换四角）计算平移量并左乘平移仿射，不触碰形状
//! 点阵。布局是构图排错的直观途径，不涉及动画调度。

use crate::Item;
use kurbo::{Affine, Rect, Vec2};

/// 视觉方向（屏幕 y 向下：`Up` = 画布上方 = y 较小侧）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 上方。
    Up,
    /// 下方。
    Down,
    /// 左侧。
    Left,
    /// 右侧。
    Right,
}

impl Item {
    /// 相对布局（manim `next_to`）：平移 self 到 `target` 包围盒 `dir` 侧邻接处
    /// （间距 `buff`，另一轴中心对齐），返回平移后的 self。
    ///
    /// 基于 bbox 计算，对任意已变换对象成立；与 [`Item::with_transform`] 一样
    /// 是 builder 风格（原对象不变）。
    pub fn next_to(self, target: &Item, dir: Direction, buff: f64) -> Self {
        let mine = self.bbox();
        let other = target.bbox();
        let delta = match dir {
            Direction::Left => {
                let dx = other.x0 - buff - mine.x1;
                let dy = (other.y0 + other.y1 - mine.y0 - mine.y1) / 2.0;
                log::info!(
                    "next_to[Left]: 自身右缘 x1={:.1} → 目标左缘 x0={:.1} - buff={:.1}（dx={dx:.2}）；y 居中对齐（dy={dy:.2}）",
                    mine.x1,
                    other.x0,
                    buff
                );
                Vec2::new(dx, dy)
            }
            Direction::Right => {
                let dx = other.x1 + buff - mine.x0;
                let dy = (other.y0 + other.y1 - mine.y0 - mine.y1) / 2.0;
                log::info!(
                    "next_to[Right]: 自身左缘 x0={:.1} → 目标右缘 x1={:.1} + buff={:.1}（dx={dx:.2}）；y 居中对齐（dy={dy:.2}）",
                    mine.x0,
                    other.x1,
                    buff
                );
                Vec2::new(dx, dy)
            }
            Direction::Up => {
                let dx = (other.x0 + other.x1 - mine.x0 - mine.x1) / 2.0;
                let dy = other.y0 - buff - mine.y1;
                log::info!(
                    "next_to[Up]: 自身下缘 y1={:.1} → 目标上缘 y0={:.1} - buff={:.1}（dy={dy:.2}）；x 居中对齐（dx={dx:.2}）",
                    mine.y1,
                    other.y0,
                    buff
                );
                Vec2::new(dx, dy)
            }
            Direction::Down => {
                let dx = (other.x0 + other.x1 - mine.x0 - mine.x1) / 2.0;
                let dy = other.y1 + buff - mine.y0;
                log::info!(
                    "next_to[Down]: 自身上缘 y0={:.1} → 目标下缘 y1={:.1} + buff={:.1}（dy={dy:.2}）；x 居中对齐（dx={dx:.2}）",
                    mine.y0,
                    other.y1,
                    buff
                );
                Vec2::new(dx, dy)
            }
        };
        let t = Affine::translate(delta) * self.transform;
        let out = self.with_transform(t);
        log::info!(
            "next_to 完成: dir={dir:?} buff={buff} 自身bbox={mine:?} 目标bbox={other:?} → 平移=({:.2},{:.2}) 结果bbox={:?}",
            delta.x,
            delta.y,
            out.bbox()
        );
        out
    }

    /// 绝对布局（manim `to_edge`）：平移 self 到画布 `dir` 侧边缘（间距 `buff`，
    /// 另一轴对画布居中——与 manim 相对帧边做 next_to 的语义一致）。
    ///
    /// `canvas` = (宽, 高) 像素；返回平移后的 self。
    pub fn to_edge(self, dir: Direction, buff: f64, canvas: (f64, f64)) -> Self {
        let mine = self.bbox();
        let (cx, cy) = ((mine.x0 + mine.x1) / 2.0, (mine.y0 + mine.y1) / 2.0);
        let delta = match dir {
            Direction::Up => {
                let d = Vec2::new(canvas.0 / 2.0 - cx, buff - mine.y0);
                log::info!(
                    "to_edge[Up]: 顶缘 y0={:.1} → buff={:.1}（dy={:.2}）；x 对画布宽 {:.0} 居中（dx={:.2}）",
                    mine.y0,
                    buff,
                    d.y,
                    canvas.0,
                    d.x
                );
                d
            }
            Direction::Down => {
                let d = Vec2::new(canvas.0 / 2.0 - cx, canvas.1 - buff - mine.y1);
                log::info!(
                    "to_edge[Down]: 底缘 y1={:.1} → 画布高 {:.0} - buff={:.1}（dy={:.2}）；x 对画布宽 {:.0} 居中（dx={:.2}）",
                    mine.y1,
                    canvas.1,
                    buff,
                    d.y,
                    canvas.0,
                    d.x
                );
                d
            }
            Direction::Left => {
                let d = Vec2::new(buff - mine.x0, canvas.1 / 2.0 - cy);
                log::info!(
                    "to_edge[Left]: 左缘 x0={:.1} → buff={:.1}（dx={:.2}）；y 对画布高 {:.0} 居中（dy={:.2}）",
                    mine.x0,
                    buff,
                    d.x,
                    canvas.1,
                    d.y
                );
                d
            }
            Direction::Right => {
                let d = Vec2::new(canvas.0 - buff - mine.x1, canvas.1 / 2.0 - cy);
                log::info!(
                    "to_edge[Right]: 右缘 x1={:.1} → 画布宽 {:.0} - buff={:.1}（dx={:.2}）；y 对画布高 {:.0} 居中（dy={:.2}）",
                    mine.x1,
                    canvas.0,
                    buff,
                    d.x,
                    canvas.1,
                    d.y
                );
                d
            }
        };
        let t = Affine::translate(delta) * self.transform;
        let out = self.with_transform(t);
        log::info!(
            "to_edge 完成: dir={dir:?} buff={buff} 画布={canvas:?} 自身bbox={mine:?} → 平移=({:.2},{:.2}) 结果bbox={:?}",
            delta.x,
            delta.y,
            out.bbox()
        );
        out
    }
}

/// 批量布局（manim `arrange`）：`items` 沿 `dir` 依次排列——首项不动，后续每项
/// `next_to` 前一项（间距 `buff`，另一轴中心对齐），原地修改切片并返回总包围盒。
///
/// 总包围盒常用于整组再平移（如画布居中）。
pub fn arrange(items: &mut [Item], dir: Direction, buff: f64) -> Rect {
    assert!(!items.is_empty(), "arrange 需要至少一个对象");
    log::info!("arrange 开始: dir={dir:?} buff={buff} 项数={}", items.len());
    for i in 1..items.len() {
        let prev = items[i - 1].clone();
        let cur = items[i].clone();
        log::info!("arrange[第 {i} 项]:");
        items[i] = cur.next_to(&prev, dir, buff);
    }
    let total = items
        .iter()
        .map(Item::bbox)
        .reduce(|a, b| a.union(b))
        .expect("非空切片必有包围盒");
    log::info!(
        "arrange 完成: dir={dir:?} 项数={} 总bbox={total:?}",
        items.len()
    );
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    /// (0,0)-(100,60) 矩形。
    fn rect() -> Item {
        Item::rect(kurbo::Rect::new(0.0, 0.0, 100.0, 60.0))
    }

    #[test]
    fn 相对布局_右侧邻接且垂直居中() {
        let other = rect().with_transform(Affine::translate((300.0, 100.0)));
        let c = rect().next_to(&other, Direction::Right, 20.0);
        let bb = c.bbox();
        // x：other.x1=400 + buff 20 = 420
        assert!(
            (bb.x0 - 420.0).abs() < 1e-9 && (bb.x1 - 520.0).abs() < 1e-9,
            "{bb:?}"
        );
        // y：other 中心 130，mine 中心对齐 → 100..160
        assert!(
            (bb.y0 - 100.0).abs() < 1e-9 && (bb.y1 - 160.0).abs() < 1e-9,
            "{bb:?}"
        );
    }

    #[test]
    fn 相对布局_上方邻接() {
        let other = rect().with_transform(Affine::translate((300.0, 100.0)));
        let c = rect().next_to(&other, Direction::Up, 10.0);
        let bb = c.bbox();
        // y：other.y0=100 - buff 10 = 90 = mine.y1
        assert!(
            (bb.y0 - 30.0).abs() < 1e-9 && (bb.y1 - 90.0).abs() < 1e-9,
            "{bb:?}"
        );
        // x：other 中心 350 居中 → 300..400
        assert!(
            (bb.x0 - 300.0).abs() < 1e-9 && (bb.x1 - 400.0).abs() < 1e-9,
            "{bb:?}"
        );
    }

    #[test]
    fn 绝对布局_贴边且另一轴居中() {
        let c = rect().to_edge(Direction::Up, 10.0, (960.0, 540.0));
        let bb = c.bbox();
        assert!((bb.y0 - 10.0).abs() < 1e-9, "{bb:?}");
        assert!((bb.x0 - 430.0).abs() < 1e-9, "x 应居中 (960-100)/2: {bb:?}");
        let l = rect().to_edge(Direction::Left, 5.0, (960.0, 540.0));
        let bb = l.bbox();
        assert!((bb.x0 - 5.0).abs() < 1e-9, "{bb:?}");
        assert!((bb.y0 - 240.0).abs() < 1e-9, "y 应居中 (540-60)/2: {bb:?}");
    }

    #[test]
    fn 批量布局_竖排与总包围盒() {
        // 第二项带初始平移：布局按实际 bbox 计算，居中对齐后 x 归拢到首项
        let mut items = vec![
            rect(),
            rect().with_transform(Affine::translate((50.0, 0.0))),
            rect(),
        ];
        let total = arrange(&mut items, Direction::Down, 15.0);
        assert!(
            (total.x0 - 0.0).abs() < 1e-9 && (total.x1 - 100.0).abs() < 1e-9,
            "{total:?}"
        );
        // 首项 y 0..60，第二项 75..135，第三项 150..210
        assert!((total.y1 - 210.0).abs() < 1e-9, "{total:?}");
        let bb2 = items[1].bbox();
        assert!((bb2.y0 - 75.0).abs() < 1e-9, "第二项应贴首项下缘: {bb2:?}");
    }

    #[test]
    fn 相对布局_不修改原对象() {
        let other = rect().with_transform(Affine::translate((300.0, 0.0)));
        let base = rect();
        let _ = base.clone().next_to(&other, Direction::Right, 0.0);
        assert!(
            (base.bbox().x1 - 100.0).abs() < 1e-9,
            "原对象不应被布局修改: {:?}",
            base.bbox()
        );
    }
}
