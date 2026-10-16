# 基础场景族：动画词汇 / 正弦波 / 几何基元 / 布局 / 强调 / 语法糖 / 移动 / 形状变换。

运行：`cargo run -p ffanim --example basic -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## demo_anim

M2 验收场景：三角形描边生长 → 圆形滑入 → 圆形形变放大 → 文字淡入。

<video controls loop muted preload="metadata" src="media/demo_anim.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:demo_anim}}
```

</details>

## sine_demo

M4 验收场景：坐标系 + 正弦函数图像随 ValueTracker 动画（求值期信号）。

<video controls loop muted preload="metadata" src="media/sine_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:sine_demo}}
```

</details>

## primitives_demo

manim 对齐 P0 验收场景：标注基元（线/箭头/虚线/弧/椭圆）+ 位姿动作糖。

<video controls loop muted preload="metadata" src="media/primitives_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:primitives_demo}}
```

</details>

## layout_demo

manim 对齐 P0-2 验收场景：相对布局 next_to / to_edge / arrange。

<video controls loop muted preload="metadata" src="media/layout_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:layout_demo}}
```

</details>

## emphasize_demo

M9.5 验收场景：强调四件套（indicate / circumscribe / flash / show_passing_flash）。

<video controls loop muted preload="metadata" src="media/emphasize_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:emphasize_demo}}
```

</details>

## sugar_demo

M9.7 验收场景：P1 薄糖批（Uncreate / Together / Succession / 方向家族 /

<video controls loop muted preload="metadata" src="media/sugar_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:sugar_demo}}
```

</details>

## move_demo

M9.10 验收场景（P1-4）：MoveAlongPath 沿路径移动。

<video controls loop muted preload="metadata" src="media/move_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:move_demo}}
```

</details>

## shapes_demo

M9.11 验收场景（P1-5）：TransformMatchingShapes 任意片段结构匹配形变。

<video controls loop muted preload="metadata" src="media/shapes_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:shapes_demo}}
```

</details>

## riemann_area

GraphAreaPlot 对齐场景：曲线下面积 + 黎曼矩形近似（纯现有 API 拼装，∫₀^π sin x dx = 2）。

<video controls loop muted preload="metadata" src="media/riemann_area.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:riemann_area}}
```

</details>

## line_graph

HeatDiagramPlot 对齐场景：折线/散点图（对标 Axes.plot_line_graph，纯现有 API 拼装）。

<video controls loop muted preload="metadata" src="media/line_graph.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:line_graph}}
```

</details>

## angle_demo

<video controls loop muted preload="metadata" src="media/angle_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:angle_demo}}
```

</details>

## circle_sine_demo

SineCurveUnitCircle 对齐场景：单位圆动点绕转 ↔ 正弦曲线逐段展开（联动）。

<video controls loop muted preload="metadata" src="media/circle_sine_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:circle_sine_demo}}
```

</details>

## warp_grid_demo

OpeningManim 对齐场景：网格逐点非线性形变（`apply_function` 对标）。

<video controls loop muted preload="metadata" src="media/warp_grid_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:warp_grid_demo}}
```

</details>

## boolean_demo

BooleanOperation 对齐场景：两椭圆 A/B 部分交叠 → 四种布尔结果轮播

<video controls loop muted preload="metadata" src="media/boolean_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:boolean_demo}}
```

</details>

## trace_demo

PointWithTrace 对齐场景：点沿方形走一圈，身后轨迹逐段生长。

<video controls loop muted preload="metadata" src="media/trace_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:trace_demo}}
```

</details>

## polygon_axes_demo

PolygonOnAxes 对齐场景：矩形贴轴随 tracker 重绘 + 面积数字联动。

<video controls loop muted preload="metadata" src="media/polygon_axes_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:polygon_axes_demo}}
```

</details>

## follow_cam_demo

FollowingGraphCamera 对齐场景：相机推近后沿曲线跟随动点平移。

<video controls loop muted preload="metadata" src="media/follow_cam_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/basic.rs:follow_cam_demo}}
```

</details>

