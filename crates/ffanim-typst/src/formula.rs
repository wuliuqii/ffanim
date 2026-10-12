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
    /// 绘制片段（字形轮廓 + 几何形状，顺序即 Typst Frame 遍历序）。
    pub pieces: Vec<FormulaPiece>,
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
