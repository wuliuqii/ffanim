# 3D 光源位置示例：ThreeDLightSourcePosition 验收（docs/07 §1.2，manim gallery 对照）。

运行：`cargo run -p ffanim --example three_d_light -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## three_d_light

ThreeDLightSourcePosition 的 ffanim 版：球面棋盘 + 三轴，静态机位展示。

<video controls loop muted preload="metadata" src="media/three_d_light.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/three_d_light.rs:three_d_light}}
```

</details>

