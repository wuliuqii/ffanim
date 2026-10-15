# 第 1 课 · 形状与样式

一切画面都从 **Item** 开始：圆、矩形、多边形……本课认识三种最常用的
基元，以及三种着色方式。完整代码不到 40 行——

<video controls loop muted preload="metadata" src="media/lesson_shapes.mp4"></video>

```rust
{{#include ../../../crates/ffanim/examples/lessons.rs:lesson_shapes}}
```

## 要点

- **`tl.add(...)`** 把 Item 注册进时间轴，返回一个句柄（`ItemId`）；
  之后所有动画都用句柄指名道姓。
- **`with_fill`** 填内部、**`with_stroke`** 勾轮廓、**`with_stroke_width`**
  定粗细。可以任意组合：只描边 = 线框，只填充 = 剪影。
- **`with_alpha(90)`** 调透明度（0 全透 ~ 255 不透），常用于半透明高亮块。
- **`tl.group([...], 0.15)`** 让一组动画同窗开始、依次错峰 0.15s——
  这就是「逐个出场」的全部秘密。
- 所有颜色取自主题（`theme.foreground()` 等），主题从 JSON 读入，
  代码里没有硬编码色值。

## 试着改一改

- 把六边形改成五边形：`regular_polygon(..., 6)` → `..., 5`
- 给矩形加填充：补一行 `.with_fill(...)`
- 把错峰改成 0.4s——出场立刻变成慢条斯理的逐个登场

下一课让这些形状动起来 →[第 2 课 · 动画词汇](lesson-anims.md)
