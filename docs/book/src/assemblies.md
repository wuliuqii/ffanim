# 装配件场景族：花括号 / 定界符 / 箭头 / 数字滚动（Typst MATH stretch 装配）。

运行：`cargo run -p ffanim --example assemblies -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## brace_demo

M9.12 验收场景（P1-8）：Brace 花括号 + 括注文本。

<video controls loop muted preload="metadata" src="media/brace_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/assemblies.rs:brace_demo}}
```

</details>

## delimiter_demo

M9.13 验收场景：Delimiter 四类定界符统一 Typst MATH 装配。

<video controls loop muted preload="metadata" src="media/delimiter_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/assemblies.rs:delimiter_demo}}
```

</details>

## arrow_demo

arrow_demo：Typst 装配箭头验收示例（M9.14）。

<video controls loop muted preload="metadata" src="media/arrow_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/assemblies.rs:arrow_demo}}
```

</details>

## decimal_demo

M9.15 验收场景：P1-6 数字滚动（manim `DecimalNumber` + ValueTracker 对应）。

<video controls loop muted preload="metadata" src="media/decimal_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/assemblies.rs:decimal_demo}}
```

</details>

