//! 渲染后端选择器：GPU 优先（生产路径），失败自动降级 CPU（兜底路径）。
//!
//! 架构约束（docs/02-架构设计.md §3.4）：两个后端消费同一 `DrawList`；
//! [`FrameBackend::auto`] 在 gpu feature 开启时探测适配器，成功即用 GPU，
//! 无 GPU/驱动缺失/feature 关闭一律落 CPU——调用方无需分支。

#[cfg(feature = "gpu")]
use crate::GpuBackend;
use crate::{CpuBackend, RenderBackend};
use ffanim_core::DrawList;

/// 帧渲染后端：GPU（Vello/wgpu compute）或 CPU（vello_cpu）。
///
/// 枚举而非 trait object：两后端均无状态泛化需求，枚举保持 `Send`
/// （vello Renderer 是 Send；wgpu Device/Queue 亦然），可放入
/// `Mutex` 供出片管线的并行帧闭包共享。
pub enum FrameBackend {
    /// GPU 生产路径（`gpu` feature 下可用）。装箱：GpuBackend 持有
    /// wgpu 设备 + vello 渲染器，体积远大于 CPU 分支（clippy lint）。
    #[cfg(feature = "gpu")]
    Gpu(Box<GpuBackend>),
    /// CPU 兜底路径（CI/无 GPU/显式指定）。
    Cpu(CpuBackend),
}

impl FrameBackend {
    /// 自动选择：探测 GPU 适配器，成功即 GPU（打日志），否则 CPU。
    ///
    /// 探测只发生在这里一次；失败原因打 `log::warn`（无 GPU 是常态而非错误）。
    pub fn auto() -> Self {
        #[cfg(feature = "gpu")]
        {
            match GpuBackend::new() {
                Ok(gpu) => {
                    log::info!("渲染后端: GPU（Vello）");
                    Self::Gpu(Box::new(gpu))
                }
                Err(reason) => {
                    log::warn!("GPU 不可用（{reason}），降级 CPU 后端");
                    Self::Cpu(CpuBackend::new())
                }
            }
        }
        #[cfg(not(feature = "gpu"))]
        {
            log::info!("渲染后端: CPU（gpu feature 未开启）");
            Self::Cpu(CpuBackend::new())
        }
    }

    /// 显式 CPU（像素级断言/回归基准等需要确定性的场景）。
    pub fn cpu() -> Self {
        Self::Cpu(CpuBackend::new())
    }

    /// 后端名（诊断/打印用）。
    pub fn name(&self) -> &'static str {
        match self {
            #[cfg(feature = "gpu")]
            Self::Gpu(_) => "gpu",
            Self::Cpu(_) => "cpu",
        }
    }
}

impl RenderBackend for FrameBackend {
    fn render(&mut self, draw: &DrawList) -> Vec<u8> {
        match self {
            #[cfg(feature = "gpu")]
            Self::Gpu(gpu) => gpu.render(draw),
            Self::Cpu(cpu) => cpu.render(draw),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_选择不panic且可渲染() {
        // 无 GPU 环境（CI）自动落 CPU；有 GPU 环境（本机）走 GPU——两种均须出图
        let mut backend = FrameBackend::auto();
        eprintln!("实际后端: {}", backend.name());
        let mut draw = DrawList::new(32, 32, ffanim_core::Color::new(14, 17, 22, 255));
        draw.push(
            ffanim_core::Item::circle(kurbo::Circle::new((16.0, 16.0), 8.0))
                .with_fill(ffanim_core::Color::new(255, 0, 0, 255)),
        );
        let buf = backend.render(&draw);
        assert_eq!(buf.len(), 32 * 32 * 4);
        // 圆心红色（两后端一致性冒烟）
        let center = ((16 * 32 + 16) * 4) as usize;
        assert!(
            buf[center] > 200,
            "圆心应为红色: {:?}",
            &buf[center..center + 4]
        );
    }

    #[test]
    fn cpu_显式路径确定性() {
        let draw = {
            let mut d = DrawList::new(16, 16, ffanim_core::Color::new(0, 0, 0, 255));
            d.push(
                ffanim_core::Item::rect(kurbo::Rect::new(0.0, 0.0, 8.0, 8.0))
                    .with_fill(ffanim_core::Color::new(255, 255, 255, 255)),
            );
            d
        };
        let a = FrameBackend::cpu().render(&draw);
        let b = FrameBackend::cpu().render(&draw);
        assert_eq!(a, b, "CPU 后端逐字节确定");
    }
}
