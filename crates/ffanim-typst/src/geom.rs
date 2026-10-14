//! 装配符号几何工具：Typst pt 排版空间 → 画布 px 空间的公共变换原语。
//!
//! 装配符号（arrow / delimiter / 后续 P1 系列）共用管线 =「Typst 排版（pt，y 向下）
//! → 装配片合并 → 归一化规范形 → 画布摆位」。本模块只收**跨符号复用**的原子件；
//! 各符号特有的归一化（如 delimiter 的 rotate −90° 与档位离散精调）留在各自文件。

use ffanim_core::kurbo::{Affine, Point};

/// pt → px（Typst 排版 pt 空间 → 画布 px 空间；1pt = 96/72 px）。
pub(crate) const PT_TO_PX: f64 = 96.0 / 72.0;
/// px → pt（Typst 无 px 单位，stretch/lr 的 size 参数以 pt 表达）。
pub(crate) const PX_TO_PT: f64 = 72.0 / 96.0;

/// pt y 向下 → px y 向下的等比翻转缩放原子（x 等比放大、y 反号放大）。
///
/// 装配路径自 Typst pt 空间取出后先过此件，再做符号特有的平移/旋转归一化
/// （等价 `Affine::new([PT_TO_PX, 0, 0, -PT_TO_PX, 0, 0])`）。
pub(crate) fn pt_y_down_to_px() -> Affine {
    Affine::new([PT_TO_PX, 0.0, 0.0, -PT_TO_PX, 0.0, 0.0])
}

/// 弦角复合：规范水平形（原点 = from、杆沿 +x、中线 y = 0、头在 +x 端）
/// → 画布两点式（from → to）。
///
/// 显式 6 元矩阵（c = dx/span, s = dy/span；kurbo 列序 x' = a·x + c·y + e /
/// y' = b·x + d·y + f；画布 y 向下系顺时针正角），端点精确闭合：规范形点
/// (span, 0) → (from.x + dx, from.y + dy) = to。
///
/// 调用方须保证 from ≠ to（装配 API 均有跨度下限校验，如 arrow 的 20px）。
pub(crate) fn chord_affine(from: Point, to: Point) -> Affine {
    let d = to - from;
    let span = d.hypot();
    let (c, s) = (d.x / span, d.y / span);
    Affine::new([c, s, -s, c, from.x, from.y])
}
