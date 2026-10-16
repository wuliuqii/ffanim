# 文字与公式场景族：公式 Write/TransformMatchingTex / 富文本 / 标注复合件 / 代码高亮 / t2c 按词着色。

运行：`cargo run -p ffanim --example typography -- [场景名...]`（缺省全渲染，≙ `manim -a`）

## formula_demo

M3 验收场景：公式逐字形写出（Write）→ 结构匹配形变（TransformMatchingTex）。

<video controls loop muted preload="metadata" src="media/formula_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/typography.rs:formula_demo}}
```

</details>

## text_demo

M8.2 验收场景：Text 富文本——中英混排限宽不出框 + 粗细混排 + 行距。

<video controls loop muted preload="metadata" src="media/text_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/typography.rs:text_demo}}
```

</details>

## annotate_demo

manim 对齐 P1-7 验收场景：标注复合件（圈选框/下划线/叉/背景板）。

<video controls loop muted preload="metadata" src="media/annotate_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/typography.rs:annotate_demo}}
```

</details>

## code_demo

M9.9 验收场景（P1-13）：代码块语法高亮。

<video controls loop muted preload="metadata" src="media/code_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/typography.rs:code_demo}}
```

</details>

## t2c_demo

M9.16 验收场景：P1-12 按词着色（manim `Text(t2c=…)` 对应）。

<video controls loop muted preload="metadata" src="media/t2c_demo.mp4"></video>

<details>
<summary>完整源码</summary>

```rust
{{#include ../../../crates/ffanim/examples/typography.rs:t2c_demo}}
```

</details>

