# 第 6 课 · 镜头语言

相机本身就是一个可动画对象：**注视点 + 缩放 + 旋转**。`transform_to`
把它推到新取景，观众就完成一次「推近」或「拉远」。

<video controls loop muted preload="metadata" src="media/lesson_camera.mp4"></video>

```rust
{{#include ../../../crates/ffanim/examples/lessons.rs:lesson_camera}}
```

## 要点

- **`tl.camera(Camera::at(960, 540))`** 注册默认相机（注视画布中心、
  zoom=1.0，此时取景恰与无相机一致）。
- **`Camera { center, zoom, rotation }`** 三个字段完整描述取景：
  `center` 是注视点的**世界坐标**（用 `axes.c2p` 算，与场景对象同一
  坐标系）、`zoom>1` 推近、`<1` 拉远。
- **`Anim::transform_to(camera, 目标.affine(w, h))`** 是相机唯一需要
  的动画——`affine(w, h)` 把相机描述编译成 world→screen 仿射矩阵。
- 相机与普通对象可以同窗编排（`tl.together`）：相机跟随动点巡行的
  完整示例见 [follow_cam_demo](basic.md#follow_cam_demo)。

## 试着改一改

- `zoom: 1.8` 改成 `0.7`——拉远出黑边（相机比画布看得更广）
- 注视点改到曲线左峰：`peak_x = -1.2`
- 加一段 `rotation: 0.3` 的目标——斜着看世界

## 六课毕业

到这里你已经掌握了 ffanim 的全部核心概念：**Item**（画什么）、
**Anim**（怎么变）、**Timeline**（何时变）、**tracker + updater**
（数据驱动）、**Axes**（坐标换算）、**Camera**（取景）。

接下来：

- 回[效果画廊](effect-gallery.md)把眼熟的效果逐个对号入座
- 去[场景画廊](basic.md)读真实创作场景的完整源码
- 需要 manim 词汇对照，查[附录](manim-gallery.md)
