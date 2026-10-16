# 表格场景族：core 侧网格拼装（TableSpec 三通道）+ 行组错峰入场 + 高亮底板标注。

运行：`cargo run -p ffanim --example tables -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## table_demo

M9.18 验收场景：P1-14 表格（manim `Table` 对应）。

<video controls loop muted preload="metadata" src="media/table_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/tables.rs:table_demo}}
```

</details>

