//! 排版产物模型：`FormulaOptions` / `FormulaPiece` / `Formula` 与摆放变换。

use ffanim_core::kurbo::{Affine, BezPath};
use ffanim_core::{Color, Item};

/// 公式排版选项。
#[derive(Debug, Clone)]
pub struct FormulaOptions {
    /// 正文字号（pt；数学字号与正文一致）。
    pub font_size_pt: f64,
    /// 文字填充色（数据驱动：来自主题）。
    pub text_color: Color,
}

/// 公式中的一个绘制片段（字形轮廓或几何形状）。
#[derive(Debug, Clone)]
pub struct FormulaPiece {
    /// 绝对坐标路径（pt，y 向下，已含字形平移与缩放）。
    pub path: BezPath,
    /// 填充色（None = 只描边，如分数线）。
    pub fill: Option<Color>,
    /// (描边色, 描边宽度 pt)。
    pub stroke: Option<(Color, f64)>,
    /// 源码字节区间（主源内），供结构匹配。
    pub source_range: Option<(usize, usize)>,
    /// 该片段对应的源文本（字形才有；分数线等形状为 None）。
    /// TransformMatchingTex 的匹配依据。
    pub text: Option<String>,
}

/// 排版完成的公式（不可变，可被多帧共享）。
#[derive(Debug, Clone)]
pub struct Formula {
    /// 排版宽度（pt）。
    pub width_pt: f64,
    /// 排版高度（pt）。
    pub height_pt: f64,
    /// 排版主字号（pt），供像素意图落位换算（[`Formula::to_items_at`]）。
    pub font_size_pt: f64,
    /// 绘制片段（字形轮廓 + 几何形状，顺序即 Typst Frame 遍历序）。
    pub pieces: Vec<FormulaPiece>,
}

/// 共享排版选项构造：白填充（颜色不影响轮廓几何，探针装配用）。
pub(crate) fn formula_opts(font_size_pt: f64) -> FormulaOptions {
    FormulaOptions {
        font_size_pt,
        text_color: Color::new(255, 255, 255, 255),
    }
}

impl Formula {
    /// 适配为 ffanim 对象列表（坐标原点即公式左上角，位置由场景层摆放）。
    pub fn to_items(&self) -> Vec<Item> {
        self.pieces
            .iter()
            .map(|p| {
                let mut item = Item::from_path(p.path.clone());
                if let Some(c) = p.fill {
                    item = item.with_fill(c);
                }
                if let Some((c, w)) = p.stroke {
                    item = item.with_stroke(c).with_stroke_width(w);
                }
                item
            })
            .collect()
    }

    /// 像素意图落位：一步得到「屏上主字号 ≈ `font_px`、锚点对准 `(cx, cy)`」
    /// 的对象列表（内部完成缩放换算与拆件落位，替代
    /// `to_items → fit_transform → map(with_transform) → add` 四拍）。
    ///
    /// 锚点为 [`Anchor::Center`] 时 (cx, cy) 是缩放后公式的**中心**；
    /// [`Anchor::Top`] 时是**顶边中点**（标题/字幕顶锚定排版常用）。
    pub fn to_items_at(&self, cx: f64, cy: f64, font_px: f64, anchor: Anchor) -> Vec<Item> {
        // 屏上字号 px = font_size_pt × scale ⟹ scale = font_px / font_size_pt
        let scale = font_px / self.font_size_pt;
        let (tx, ty) = match anchor {
            Anchor::Center => (
                cx - scale * self.width_pt / 2.0,
                cy - scale * self.height_pt / 2.0,
            ),
            Anchor::Top => (
                cx - scale * self.width_pt / 2.0,
                cy, // 顶边锚定：缩放后顶边恰在 cy
            ),
        };
        let place = Affine::translate((tx, ty)) * Affine::scale(scale);
        self.to_items()
            .into_iter()
            .map(|item| item.with_transform(place))
            .collect()
    }
}

/// 落位锚点意图：`(cx, cy)` 指向缩放后公式的哪个点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    /// 中心锚定（默认）：公式中心落在目标点。
    #[default]
    Center,
    /// 顶边中点锚定：公式顶边中点落在目标点（标题带、字幕行常用）。
    Top,
}

/// 供测试/诊断：把公式重新居中到矩形（后续场景摆放用例）。
impl Formula {
    /// 生成把公式中心移到 (cx, cy) 的仿射变换。
    pub fn center_transform(&self, cx: f64, cy: f64) -> Affine {
        Affine::translate((cx - self.width_pt / 2.0, cy - self.height_pt / 2.0))
    }

    /// 缩放后居中：先按 `scale` 缩放，再让**缩放后**的公式中心落在 (cx, cy)。
    ///
    /// 注意 `center_transform(cx, cy) * Affine::scale(s)` 是错的——居中按未缩放
    /// 尺寸计算，实际中心会偏移 (s-1)/2·(w,h)，横向溢出/出画布均源于此。
    pub fn fit_transform(&self, cx: f64, cy: f64, scale: f64) -> Affine {
        Affine::translate((
            cx - scale * self.width_pt / 2.0,
            cy - scale * self.height_pt / 2.0,
        )) * Affine::scale(scale)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 100×50pt、主字号 25pt 的测试公式（单片段空白路径）。
    fn sample() -> Formula {
        Formula {
            width_pt: 100.0,
            height_pt: 50.0,
            font_size_pt: 25.0,
            pieces: vec![FormulaPiece {
                path: BezPath::new(),
                fill: Some(Color::new(0, 0, 0, 255)),
                stroke: None,
                source_range: None,
                text: None,
            }],
        }
    }

    #[test]
    fn 像素意图落位_中心锚() {
        // 25pt 公式落 50px 屏上字号 → scale = 2.0，中心 (480, 158)
        let f = sample();
        let items = f.to_items_at(480.0, 158.0, 50.0, Anchor::Center);
        assert_eq!(items.len(), 1);
        let [a, _, _, _, e, g] = items[0].transform.as_coeffs();
        assert_eq!(a, 2.0, "scale = font_px/font_size_pt");
        // 中心落位：e = cx - scale·w/2 = 480 - 100
        assert!((e - (480.0 - 2.0 * 100.0 / 2.0)).abs() < 1e-9);
        assert!((g - (158.0 - 2.0 * 50.0 / 2.0)).abs() < 1e-9);
    }

    #[test]
    fn 像素意图落位_顶锚() {
        // 26pt 公式落 26px → scale 1.0，顶边锚在 495
        let f = sample();
        let items = f.to_items_at(480.0, 495.0, 26.0, Anchor::Top);
        let [a, _, _, _, e, g] = items[0].transform.as_coeffs();
        assert_eq!(a, 26.0 / 25.0);
        assert!((e - (480.0 - 26.0 / 25.0 * 100.0 / 2.0)).abs() < 1e-9);
        assert_eq!(g, 495.0, "顶边锚定：ty 恰为 cy");
    }
}
