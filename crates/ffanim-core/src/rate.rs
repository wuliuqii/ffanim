//! 缓动函数：manim rate_func 词汇的等价实现。
//!
//! 所有函数输入输出均约束在 [0,1]，超界输入先被钳制。

/// 缓动类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RateFunc {
    /// 线性。
    Linear,
    /// 平滑缓入缓出（manim 默认 `smooth`，smoothstep）。
    #[default]
    Smooth,
    /// 缓入。
    EaseIn,
    /// 缓出。
    EaseOut,
    /// 去而复返（先到 1 再回到 0），适合"指向后收回"的手势类动画。
    ThereAndBack,
}

impl RateFunc {
    /// 求值：t ∈ [0,1] → 进度 ∈ [0,1]（ThereAndBack 中点后回落）。
    pub fn eval(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Self::Linear => t,
            Self::Smooth => t * t * (3.0 - 2.0 * t),
            Self::EaseIn => t * t * t,
            Self::EaseOut => 1.0 - (1.0 - t).powi(3),
            // 二次缓动拼接：前半段 0→1，后半段 1→0
            Self::ThereAndBack => {
                if t < 0.5 {
                    Self::Smooth.eval(t * 2.0)
                } else {
                    Self::Smooth.eval((1.0 - t) * 2.0)
                }
            }
        }
    }
}

/// 线性插值。
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 端点与中点() {
        for f in [
            RateFunc::Linear,
            RateFunc::Smooth,
            RateFunc::EaseIn,
            RateFunc::EaseOut,
        ] {
            assert_eq!(f.eval(0.0), 0.0);
            assert_eq!(f.eval(1.0), 1.0);
        }
        assert_eq!(RateFunc::Linear.eval(0.5), 0.5);
        assert_eq!(RateFunc::ThereAndBack.eval(0.0), 0.0);
        assert_eq!(RateFunc::ThereAndBack.eval(0.5), 1.0);
        assert_eq!(RateFunc::ThereAndBack.eval(1.0), 0.0);
        // 越界钳制
        assert_eq!(RateFunc::Smooth.eval(-0.5), 0.0);
        assert_eq!(RateFunc::Smooth.eval(1.5), 1.0);
    }
}
