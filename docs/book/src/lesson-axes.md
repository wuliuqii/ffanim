# 第 4 课 · 坐标系与函数图像

**Axes** 管两件事：画坐标轴，以及把**数据坐标**换算成**画布像素**。
函数图像 + tracker 驱动的动点，是数学动画的核心桥段。

<video controls loop muted preload="metadata" src="media/lesson_axes.mp4"></video>

```rust
{{#include ../../../crates/ffanim/examples/lessons.rs:lesson_axes}}
```

## 要点

- **`Axes::new(x 范围, y 范围)`** 声明数据坐标系；`.with_size` 定占屏
  大小、`.with_center` 定画布位置；`axes.to_item(&theme.style)` 转成
  可注册的 Item。
- **`axes.plot(f, 范围)`** 直接吃一个闭包——传任何 `f64 → f64` 函数
  就能得到曲线 Item。
- **tracker** 是一个可动画的数：`tl.add_tracker(0.0)` 创建，
  `Anim::set_value(t, 1.0)` 让它从当前值滑到目标值。
- **updater** 绑定 tracker：`tl.add_updater(&[t], move |v| Item)` 里
  每帧用 `v[0]`（tracker 当前值）**重建**一个 Item。动点、跟随标注、
  动态图像全靠这一个原语。
- `axes.c2p(x, y)`（coordinate to point）把数据坐标换算成画布像素——
  updater 里构造跟随对象必用它。

## 试着改一改

- 换函数：闭包改成 `|x| 0.8 * (0.5 * x * x).sin()`
- 动点变成小竖线：updater 里改返回 `Item::line(axes.c2p(x,0.0), axes.c2p(x,f(x)), ...)`——
  像黎曼积分的「扫描线」
- 让 tracker 来回：`set_value(t,1.0)` 之后再加 `set_value(t,0.0)`

下一课排版与强调 →[第 5 课 · 布局与强调](lesson-layout.md)
