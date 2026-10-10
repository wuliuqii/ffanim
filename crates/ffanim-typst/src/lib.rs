//! ffanim-typst：Typst 嵌入（World 实现）、排版缓存、`Frame → Item` 适配。
//!
//! 设计约束：本 crate 是唯一接触 Typst API 的地方，锁死版本以隔离上游破坏性变更。

pub fn placeholder() -> &'static str {
    "ffanim-typst: M3 实现（World / 排版缓存 / Frame→Items 适配）"
}
