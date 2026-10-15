//! CLI 子命令实现：clap 定义留在 main.rs，每个 handler 是 `pub fn run_*`。
//!
//! 场景绑定子命令（render/timeline/inspect/snapshot/preview/frame/strip/onion）
//! 已随 scenes.rs 迁移删除；场景示范见门面 crate examples/。
//! 保留模块：
//! - scaffold：new / info（脚手架与元信息）
//! - audio：音轨诊断
//! - formula：diff 公式片段匹配预检

pub mod audio;
pub mod formula;
pub mod scaffold;
