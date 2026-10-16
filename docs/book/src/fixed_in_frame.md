# 固定屏上层：FixedInFrameMObjectTest 验收（docs/07 §1.2，manim gallery 对照收官例）。

运行：`cargo run -p ffanim --example fixed_in_frame -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## fixed_in_frame

FixedInFrameMObjectTest 的 ffanim 版：3D 层动、2D 层固定，对照收官。

<video controls loop muted preload="metadata" src="media/fixed_in_frame.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/fixed_in_frame.rs:fixed_in_frame}}
```

</details>

