# 第 5 课 · 布局与强调

布局是**构建期**操作（不算动画）：先算好摆位再注册；强调是**渲染期**
动画，一句话锁定观众视线。

<video controls loop muted preload="metadata" src="media/lesson_layout.mp4"></video>

```rust
{{#include ../../../crates/ffanim/examples/lessons.rs:lesson_layout}}
```

## 要点

- **`arrange(&mut blocks, Direction::Down, 50.0)`** 让一组 Item 沿方向
  排队（间隔 50px），返回**总包围盒**——用它把整组平移到画布中心。
  `Direction::Right` 就是横排。
- **`f.center_transform(0.0, 0.0)`** 先把每块公式聚到原点附近再排队，
  是「先排版、后落位」的标准两步走。
- **复合变换要手写合成**：`it.transform = shift * it.transform`。
  `with_transform` 是**替换**而非叠加——初学最容易踩的坑。
- **`tl.indicate(id, 1.08, 0.6)`** 轻晃提醒（factor=放大到 8%）；
  **`tl.circumscribe(id, 1.0, 14.0, accent)`** 沿外扩 14px 的矩形描边
  圈一圈。同族还有 `tl.flash`（位置闪烁）与 `tl.show_passing_flash`
  （光点沿曲线划过）。

## 试着改一改

- `Direction::Down` 改成 `Right`，间隔 50 改 80——变成横排
- `tl.indicate(ids[0], ...)`——改为强调第一个公式
- 在 `circumscribe` 后面补 `tl.flash(ids[0], 0.5, ...)`——再闪一下
  （具体参数看 [emphasize_demo 源码](basic.md#emphasize_demo)）

最后一课，驾驭镜头 →[第 6 课 · 镜头语言](lesson-camera.md)
