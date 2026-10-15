# 第 3 课 · 文字与公式

公式排版由 **Typst** 引擎完成（与 ffanim 同源的 Rust 排版系统）：
公式用 Typst math 语法，文本用 Typst markup 语法。

<video controls loop muted preload="metadata" src="media/lesson_text.mp4"></video>

```rust
{{#include ../../../crates/ffanim/examples/lessons.rs:lesson_text}}
```

## 要点

- **`TypstEngine::new(16)`** 创建排版引擎（参数是缓存容量）；中文文本
  必须先 `find_cjk_font()` 注入系统 CJK 字体，否则中文会变成「口」占位。
- **`engine.formula("e^{i pi} + 1 = 0", &fopts)`**：Typst math 语法——
  上标 `^{...}`、下标 `_(...)`、希腊字母 `pi`/`alpha`、分数 `a/b` 都直接写。
- **`engine.text("...*粗体*...", &topts)`**：markup 语法，`*…*` 粗体、
  `_…_` 斜体。
- **`f.to_items()`** 把排版结果拆成逐字形散件（才能做逐字形动画）；
  **`f.fit_transform(x, y, scale)`** 算出落位仿射（scale=2.0 是 pt→px）。
- 公式字形可用 `create` 逐字形写出（Write 效果）；文本是闭合填充字形，
  出场用 `fade_in`。

## 试着改一改

- 把公式换成 `x^2 + y^2 = z^2` 或分数 `(a+b)/c`
- 给 `formula` 的 `group` 换步长 `0.02`——写字变快
- 在下面再加一行 `engine.text(...)`——多一段文本

下一课进入函数图像 →[第 4 课 · 坐标系与函数图像](lesson-axes.md)
