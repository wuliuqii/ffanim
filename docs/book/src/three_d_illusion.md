# 3D 相机错觉振荡：ThreeDCameraIllusionRotation 验收（docs/07 §1.2，manim gallery 对照）。

运行：`cargo run -p ffanim --example three_d_illusion -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## three_d_illusion

ThreeDCameraIllusionRotation 的 ffanim 版：theta 围绕起始角 ±1 rad 正弦摆动 4s。

<video controls loop muted preload="metadata" src="media/three_d_illusion.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/three_d_illusion.rs:three_d_illusion}}
```

</details>

