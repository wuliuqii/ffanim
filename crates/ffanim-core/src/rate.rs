//! 缓动函数：manim rate_func 词汇的等价实现。
//!
//! 所有函数输入输出均约束在 \[0,1\]，超界输入先被钳制。
//! 自定义缓动经 [`Timeline::register_rate`](crate::timeline::Timeline::register_rate)
//! 注册，得到 [`RateFunc::Custom`] 索引；注册表随编译产物携带（[`RateTable`]）。

use std::sync::Arc;

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
    /// 自定义缓动（注册表索引，由 [`crate::timeline::Timeline::register_rate`] 产生）。
    ///
    /// 直接调用 [`RateFunc::eval`] 时降级为线性；生产链路经 [`RateTable::eval`]
    /// 查表求值（见 [`crate::timeline::ScheduledAnim`]）。
    Custom(u16),
}

impl RateFunc {
    /// 求值：t ∈ \[0,1\] → 进度 ∈ \[0,1\]（ThereAndBack 中点后回落）。
    ///
    /// 仅处理内置变体；`Custom` 在无表上下文降级为线性（生产链路走 [`RateTable::eval`]）。
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
            Self::Custom(_) => t,
        }
    }
}

/// 自定义缓动注册表：构建期由 [`crate::timeline::Timeline::register_rate`] 填充，
/// 编译时整体搬入 [`crate::eval::CompiledTimeline`]，渲染期并行查表（`Send + Sync`）。
#[derive(Clone, Default)]
pub struct RateTable {
    entries: Vec<Arc<dyn Fn(f64) -> f64 + Send + Sync>>,
}

impl RateTable {
    /// 注册自定义缓动，返回可挂到 [`Anim::rate`](crate::anim::Anim::rate) 的句柄。
    pub fn insert(&mut self, f: impl Fn(f64) -> f64 + Send + Sync + 'static) -> RateFunc {
        self.entries.push(Arc::new(f));
        RateFunc::Custom((self.entries.len() - 1) as u16)
    }

    /// 带表求值：内置变体走 [`RateFunc::eval`]，`Custom` 按注册索引查表。
    /// 索引越界（不可能由正常流程产生）降级线性并断言。
    pub fn eval(&self, rate: RateFunc, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match rate {
            RateFunc::Custom(id) => match self.entries.get(id as usize) {
                Some(f) => f(t).clamp(0.0, 1.0),
                None => {
                    debug_assert!(false, "RateFunc::Custom({id}) 索引越界");
                    t
                }
            },
            built_in => built_in.eval(t),
        }
    }

    /// 已注册条数（诊断用）。
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Arc 共享函数不可逐字段 Debug，打印条数即可。
impl std::fmt::Debug for RateTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RateTable")
            .field("entries", &self.entries.len())
            .finish()
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

    #[test]
    fn 自定义缓动查表求值() {
        let mut table = RateTable::default();
        // 二次方缓入：f(t) = t²，注册即得句柄
        let square = table.insert(|t| t * t);
        assert!(matches!(square, RateFunc::Custom(0)));
        assert_eq!(table.eval(square, 0.5), 0.25);
        assert_eq!(table.eval(square, 0.0), 0.0);
        assert_eq!(table.eval(square, 1.0), 1.0);
        // 越界钳制同样生效
        assert_eq!(table.eval(square, 2.0), 1.0);
        // 多条注册索引递增
        let bump = table.insert(|t| (t * 2.0).min(1.0));
        assert!(matches!(bump, RateFunc::Custom(1)));
        assert_eq!(table.eval(bump, 0.25), 0.5);
        // 内置变体经表求值与直调一致
        assert_eq!(
            table.eval(RateFunc::Smooth, 0.5),
            RateFunc::Smooth.eval(0.5)
        );
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn custom_无表降级线性() {
        assert_eq!(RateFunc::Custom(0).eval(0.3), 0.3);
    }
}
