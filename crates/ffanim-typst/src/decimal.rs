//! 数字滚动（manim `DecimalNumber` + ValueTracker 对应）：Typst 逐帧排版。
//!
//! 机制 = tracker（求值期信号源）+ updater（每帧重建形状）+ Typst 排版。
//! 滚动时每帧新排版键（LRU 命中仅在值重复时），短公式排版亚毫秒级、
//! 逐帧重排可行；产物为闭合填充字形 → **出场用 fade_in（禁 create）**。
//!
//! 排版模式 = **markup 文本**（探针实证降级：math 模式逗号/正号带运算
//! 空隙 +14px/+24px、数字字形高度随位数波动 22→27.8px，均不可接受；
//! 文本模式 lining 数字高度一致、逗号/正号纯字形宽）。
//!
//! 中心锚定：每帧产物 bbox 中心精确落到 `anchor`——数字位数变化时
//! 视觉中心不漂移（manim DecimalNumber 居中语义对齐）。

use ffanim_core::kurbo::{Affine, BezPath, Point, Shape as _};
use ffanim_core::{Color, Error, Item, Result};

use super::engine::TypstEngine;
use super::formula::Formula;
use super::geom::PT_TO_PX;
use super::text::TextOptions;

/// 信号内部引擎的排版缓存容量（滚动时每帧新键，小容量即可）。
const SIGNAL_CACHE_CAP: usize = 8;

/// 数字排版选项（manim `DecimalNumber` 参数对齐）。
#[derive(Debug, Clone)]
pub struct DecimalNumberOptions {
    /// 小数位数（manim `num_decimal_places`）。
    pub decimals: usize,
    /// 正数显式 `+` 号（manim `include_sign`；负号恒显）。
    pub include_sign: bool,
    /// 整数部分千分位逗号（manim `group_with_commas`）。
    pub group_with_commas: bool,
    /// 字号（pt）。
    pub font_size_pt: f64,
    /// 填充色（数据驱动：来自主题）。
    pub text_color: Color,
}

impl Default for DecimalNumberOptions {
    fn default() -> Self {
        Self {
            decimals: 1,
            include_sign: false,
            group_with_commas: true,
            font_size_pt: 24.0,
            text_color: Color::new(255, 255, 255, 255),
        }
    }
}

/// 整数数字串插千分位逗号（输入须为纯 ASCII 数字）。
fn insert_commas(digits: &str) -> String {
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, b) in bytes.iter().enumerate() {
        if i > 0 && (bytes.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*b as char);
    }
    out
}

/// 值 → 数字串：先按 `10^decimals` 缩放取整（消除 `{:.N$}` 的 -0.0 与
/// 半偶歧义），`-0.0` 归零；符号与千分位按选项。
pub(crate) fn format_decimal(value: f64, o: &DecimalNumberOptions) -> String {
    let scale = 10f64.powi(o.decimals as i32);
    let rounded = (value * scale).round() / scale;
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    let neg = rounded < 0.0;
    // 定点格式化（避免科学计数法），按小数位拆整数/小数段
    let s = format!("{:.*}", o.decimals, rounded.abs());
    let (int_part, frac) = match s.split_once('.') {
        Some((i, f)) => (i.to_string(), Some(f.to_string())),
        None => (s, None),
    };
    let int_part = if o.group_with_commas {
        insert_commas(&int_part)
    } else {
        int_part
    };
    let sign = if neg {
        "-"
    } else if o.include_sign {
        "+"
    } else {
        ""
    };
    match frac {
        Some(f) if o.decimals > 0 => format!("{sign}{int_part}.{f}"),
        _ => format!("{sign}{int_part}"),
    }
}

/// 装配片合并单路径（数字为纯填充字形，同色不透明重叠区无瑕疵）。
fn assemble(f: &Formula) -> Result<BezPath> {
    let mut out = BezPath::new();
    for p in &f.pieces {
        for el in p.path.elements() {
            out.push(*el);
        }
    }
    if out.elements().is_empty() {
        return Err(Error::Other("数字装配片为空".into()));
    }
    Ok(out)
}

fn text_opts(o: &DecimalNumberOptions) -> TextOptions {
    let mut t = TextOptions::new(o.font_size_pt, o.text_color);
    t.wrap_width_pt = None; // 单行数字，禁换行
    t
}

impl TypstEngine {
    /// 数字排版：值按选项格式化后走 Typst markup 文本排版（lining 数字
    /// 高度一致、逗号/正号纯字形宽——math 模式探针实证降级，见模块注释），
    /// 片段合并单路径，bbox 中心精确落到 `anchor`（滚动换值时中心稳定）。
    ///
    /// # Errors
    /// Typst 排版失败或装配片为空时返回错误。
    pub fn decimal(
        &mut self,
        value: f64,
        anchor: Point,
        opts: &DecimalNumberOptions,
    ) -> Result<Item> {
        let body = format_decimal(value, opts);
        let f = self.text(&body, &text_opts(opts))?;
        let mut path = assemble(&f)?;
        // 归一化：bbox 中心 → anchor（pt y 向下 → px 同向缩放，无翻转；
        // kurbo 列序 `A * B` = 先 B 后 A）
        let bb = path.bounding_box();
        if !(bb.width() > 0.0 && bb.height() > 0.0) {
            return Err(Error::Other("数字装配路径为空".into()));
        }
        let c = Point::new((bb.x0 + bb.x1) / 2.0, (bb.y0 + bb.y1) / 2.0);
        path.apply_affine(
            Affine::translate((anchor.x, anchor.y))
                * Affine::scale(PT_TO_PX)
                * Affine::translate((-c.x, -c.y)),
        );
        Ok(Item::from_path(path).with_fill(opts.text_color))
    }

    /// 生成"值 → 数字对象"求值期信号（[`crate::tracker::Updater`] 信号核心）。
    ///
    /// - 信号自含排版引擎（`Mutex` 串行化逐帧排版；字体全内嵌，无需系统字体），
    ///   满足 [`crate::timeline::Timeline::add_updater`] 的
    ///   `Fn(&[f64]) -> Item + Send + Sync` 约束；取 `values[0]` 为显示值；
    /// - 构建期先以 0.0 试排一次（选项/字体问题在注册期暴露，不拖到渲染期）；
    /// - 每帧重排，bbox 中心恒在 `anchor`（滚动时视觉中心不漂移）。
    ///
    /// # Errors
    /// 构建期试排失败时返回错误。渲染期排版失败将 panic（updater 签名
    /// 无 Result；排版失败属场景配置错误，应尽早暴露而非静默跳帧）。
    pub fn decimal_signal(
        anchor: Point,
        opts: DecimalNumberOptions,
    ) -> Result<impl Fn(&[f64]) -> Item + Send + Sync + 'static> {
        let mut engine = TypstEngine::new(SIGNAL_CACHE_CAP);
        engine.decimal(0.0, anchor, &opts)?;
        let engine = std::sync::Mutex::new(engine);
        Ok(move |values: &[f64]| {
            let v = values.first().copied().unwrap_or(0.0);
            let mut engine = engine.lock().expect("数字排版引擎锁");
            engine.decimal(v, anchor, &opts).expect("数字逐帧排版")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts() -> DecimalNumberOptions {
        DecimalNumberOptions {
            decimals: 1,
            group_with_commas: true,
            ..Default::default()
        }
    }

    /// 格式化纯函数：千分位 / 负零归零 / 符号 / 进位舍入 / 零小数位。
    #[test]
    fn 数字格式化各形态() {
        let o = opts();
        assert_eq!(format_decimal(1234.5, &o), "1,234.5");
        assert_eq!(format_decimal(1234567.89, &o), "1,234,567.9");
        // -0.04 @1 位 → 舍入后为 0 → 归零去负号
        assert_eq!(format_decimal(-0.04, &o), "0.0");
        assert_eq!(format_decimal(-12.34, &o), "-12.3");
        // 半途舍入：1.25 @1 位 → 1.3（round 半远离零）
        assert_eq!(format_decimal(1.25, &o), "1.3");
        // 千分位关闭
        let plain = DecimalNumberOptions {
            group_with_commas: false,
            ..opts()
        };
        assert_eq!(format_decimal(1234.5, &plain), "1234.5");
        // 零小数位：无小数点，四舍五入到整数
        let int = DecimalNumberOptions {
            decimals: 0,
            ..opts()
        };
        assert_eq!(format_decimal(1234.5, &int), "1,235");
        assert_eq!(format_decimal(999.0, &int), "999");
        // 正号选项：负恒显、正显 +
        let signed = DecimalNumberOptions {
            include_sign: true,
            ..int
        };
        assert_eq!(format_decimal(45.0, &signed), "+45");
        assert_eq!(format_decimal(-45.0, &signed), "-45");
    }

    /// 排版：bbox 中心精确锚定，尺寸为正。
    #[test]
    fn 数字中心锚定() {
        let mut engine = TypstEngine::new(8);
        let anchor = Point::new(480.0, 270.0);
        let item = engine.decimal(1234.5, anchor, &opts()).unwrap();
        let bb = item.bbox();
        assert!(bb.width() > 0.0 && bb.height() > 0.0);
        assert!(
            ((bb.x0 + bb.x1) / 2.0 - anchor.x).abs() < 0.5
                && ((bb.y0 + bb.y1) / 2.0 - anchor.y).abs() < 0.5,
            "中心应锚定 {anchor:?}，实际 {bb:?}"
        );
    }

    /// 滚动换值：中心恒定、字号稳定（数字字形高度一致）、宽度随位数增长。
    /// （高度探针实证：数字字形 14.64–15.05pt 恒定；千分位逗号下降部
    /// 探出基线下 ~2.9pt 属正当排版事实，单独定界。）
    #[test]
    fn 数字滚动中心稳定() {
        let mut engine = TypstEngine::new(8);
        let anchor = Point::new(480.0, 270.0);
        let plain = DecimalNumberOptions {
            group_with_commas: false,
            ..opts()
        };
        let heights: Vec<f64> = [0.0, 999.9, 1234.5]
            .iter()
            .map(|&v| {
                let bb = engine.decimal(v, anchor, &plain).unwrap().bbox();
                assert!(
                    ((bb.x0 + bb.x1) / 2.0 - anchor.x).abs() < 0.5,
                    "值 {v} 中心漂移 {bb:?}"
                );
                bb.height()
            })
            .collect();
        assert!(
            (heights[0] - heights[1]).abs() < 0.5 && (heights[1] - heights[2]).abs() < 0.5,
            "数字字形高度应稳定 {heights:?}"
        );
        let w0 = engine
            .decimal(0.0, anchor, &plain)
            .unwrap()
            .bbox()
            .width();
        let w2 = engine
            .decimal(1234.5, anchor, &plain)
            .unwrap()
            .bbox()
            .width();
        assert!(w2 > w0 * 2.0, "六字符应显著宽于三字符 {w0} vs {w2}");
    }

    /// 千分位逗号下降部有界：带逗号高度增量 < 数字行高的 30%
    /// （逗号探出基线属正常字形，不得引入行高突变）。
    #[test]
    fn 逗号下降部有界() {
        let mut engine = TypstEngine::new(8);
        let anchor = Point::new(480.0, 270.0);
        let o = opts();
        let plain_h = engine
            .decimal(
                1234.5,
                anchor,
                &DecimalNumberOptions {
                    group_with_commas: false,
                    ..o.clone()
                },
            )
            .unwrap()
            .bbox()
            .height();
        let comma_h = engine
            .decimal(1234.5, anchor, &o)
            .unwrap()
            .bbox()
            .height();
        assert!(
            comma_h - plain_h < plain_h * 0.3,
            "逗号下降部增量 {:.2} 应 < 行高 30% ({:.2})",
            comma_h - plain_h,
            plain_h * 0.3
        );
    }

    /// 逗号间距探针：千分位逗号不应引入异常大空档（markup 模式逗号
    /// 为纯字形，宽度增量应明显小于一个数字字宽）。
    #[test]
    fn 千分位逗号间距不爆炸() {
        let mut engine = TypstEngine::new(8);
        let anchor = Point::new(480.0, 270.0);
        let o = opts();
        let plain_w = engine
            .decimal(
                12345.6,
                anchor,
                &DecimalNumberOptions {
                    group_with_commas: false,
                    ..o.clone()
                },
            )
            .unwrap()
            .bbox()
            .width();
        let comma_w = engine
            .decimal(12345.6, anchor, &o)
            .unwrap()
            .bbox()
            .width();
        let digit_w = engine
            .decimal(0.0, anchor, &o)
            .unwrap()
            .bbox()
            .width()
            / 3.0; // "0.0" ≈ 数字×2 + 点，取 1/3 为单数字宽上界
        assert!(
            comma_w - plain_w < digit_w,
            "逗号增量 {:.1} 应小于单数字宽 {:.1}",
            comma_w - plain_w,
            digit_w
        );
    }

    /// 正号探针：`include_sign` 的 `+` 只增符号字形宽，无运算空档。
    #[test]
    fn 正号不加运算空档() {
        let mut engine = TypstEngine::new(8);
        let anchor = Point::new(480.0, 270.0);
        let o = DecimalNumberOptions {
            decimals: 1,
            include_sign: true,
            ..Default::default()
        };
        let unsigned = engine
            .decimal(
                45.6,
                anchor,
                &DecimalNumberOptions {
                    include_sign: false,
                    ..o.clone()
                },
            )
            .unwrap()
            .bbox()
            .width();
        let signed = engine
            .decimal(45.6, anchor, &o)
            .unwrap()
            .bbox()
            .width();
        let digit_w = engine
            .decimal(0.0, anchor, &o)
            .unwrap()
            .bbox()
            .width()
            / 3.0;
        assert!(
            signed - unsigned < digit_w,
            "正号增量 {:.1} 应小于单数字宽 {:.1}",
            signed - unsigned,
            digit_w
        );
    }

    /// 信号满足 `Timeline::add_updater` 约束（Send + Sync + 'static），
    /// 且不同值的产物中心一致。
    #[test]
    fn 信号满足updater约束() {
        fn assert_signal<F: Fn(&[f64]) -> Item + Send + Sync + 'static>(_: &F) {}
        let anchor = Point::new(480.0, 270.0);
        let sig = TypstEngine::decimal_signal(anchor, opts()).unwrap();
        assert_signal(&sig);
        let a = sig(&[0.0]).bbox();
        let b = sig(&[1234.5]).bbox();
        assert!(
            ((a.x0 + a.x1) / 2.0 - anchor.x).abs() < 0.5
                && ((b.x0 + b.x1) / 2.0 - anchor.x).abs() < 0.5,
            "信号产物中心应恒在锚点 {a:?} vs {b:?}"
        );
    }
}
