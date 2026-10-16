# 3D 相机环境自转：ThreeDCameraRotation 验收（docs/07 §1.2，manim gallery 对照）。

运行：`cargo run -p ffanim --example three_d_rotation -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## three_d_rotation

ThreeDCameraRotation 的 ffanim 版：环境自转 0.1 圈/s × 3s，再 Smooth 回位。

<video controls loop muted preload="metadata" src="media/three_d_rotation.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/three_d_rotation.rs:three_d_rotation}}
```

</details>

