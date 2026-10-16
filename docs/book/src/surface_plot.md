# 3D 曲面绘制示例：ThreeDSurfacePlot 验收（docs/07 §1.2，manim gallery 对照）。

运行：`cargo run -p ffanim --example surface_plot -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## surface_plot

ThreeDSurfacePlot 的 ffanim 版：高斯峰曲面棋盘 + 三轴，静态机位展示。

<video controls loop muted preload="metadata" src="media/surface_plot.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/surface_plot.rs:surface_plot}}
```

</details>

