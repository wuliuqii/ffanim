//! CLI 子命令实现：clap 定义留在 main.rs，每个 handler 是 `pub fn run_*`。
//!
//! 模块划分（docs/04-项目优化规划.md §4.2）：
//! - render / timeline / frame（渲染同族）
//! - inspect / snapshot（诊断与基准对）
//! - strip / onion（Agent 视觉诊断，转发 ffanim_render::tools）
//! - new / info（脚手架与元信息）
//! - audio / preview

pub mod audio;
pub mod inspect;
pub mod preview;
pub mod render;
pub mod scaffold;
pub mod tools;
