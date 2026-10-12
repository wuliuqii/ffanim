//! 相机：位置/缩放/旋转的可动画描述（相机即 Item，见 docs/02-架构设计.md §3.4）。
//!
//! 相机本身注册为时间轴上的不可见对象，其 `transform` 即 world→screen 仿射；
//! 推拉摇移 = 用 [`crate::anim::Anim::transform_to`] 把相机变换到 [`Camera::affine`] 目标。

use kurbo::{Affine, Vec2};

/// 相机（注视点 / 缩放 / 旋转）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// 注视点的世界坐标（画布像素系，x 右 y 下）。
    pub center: Vec2,
    /// 缩放倍数（1.0 = 不缩放；>1 放大 = 推近，<1 缩小 = 拉远）。
    pub zoom: f64,
    /// 旋转角（弧度，kurbo 仿射语义；画布 y 向下，正值视觉上为顺时针）。
    pub rotation: f64,
}

impl Camera {
    /// 默认相机：注视画布中心、不缩放不旋转（world→screen 恰为单位阵，等价无相机）。
    pub fn at(width: u32, height: u32) -> Self {
        Self {
            center: Vec2::new(f64::from(width) / 2.0, f64::from(height) / 2.0),
            zoom: 1.0,
            rotation: 0.0,
        }
    }

    /// 构造 world→screen 仿射：注视点映射到屏幕中心，缩放与旋转以注视点为不动点。
    pub fn affine(&self, width: u32, height: u32) -> Affine {
        let sc = Vec2::new(f64::from(width) / 2.0, f64::from(height) / 2.0);
        Affine::translate(sc)
            * Affine::rotate(self.rotation)
            * Affine::scale(self.zoom)
            * Affine::translate(-self.center)
    }
}
