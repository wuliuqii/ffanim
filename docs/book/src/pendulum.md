# 单摆演示：RK4 数值积分驱动物理动画（docs/07 §2.1 ODE 能力的验收场景）。

运行：`cargo run -p ffanim --example pendulum -- [场景名...]`（缺省全渲染，≙ `manim -a`）

场景参数示范（docs/07 §5.2）：`--params x.json` 只覆盖要改的字段，如
`{"length": 2.5, "theta0_deg": 20}` 换长摆小角；缺省全用 Default 值。

## pendulum_demo

无阻尼单摆 θ'' = −(g/L)·sinθ，缺省 L=1 m、g=9.8、θ₀=40° 摆动 8 秒。

<video controls loop muted preload="metadata" src="media/pendulum_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/pendulum.rs:pendulum_demo}}
```

</details>
