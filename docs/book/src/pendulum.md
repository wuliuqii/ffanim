# 单摆演示：RK4 数值积分驱动物理动画（docs/07 §2.1 ODE 能力的验收场景）。

运行：`cargo run -p ffanim --example pendulum -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## pendulum_demo

无阻尼单摆 θ'' = −(g/L)·sinθ，从 θ₀ 释放摆动 duration 秒。

<video controls loop muted preload="metadata" src="media/pendulum_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/pendulum.rs:pendulum_demo}}
```

</details>

