# 第 2 课 · 动画词汇

ffanim 的动画是**排节目单**：`tl.play(...)` 一段接一段，`tl.wait(...)`
插入停顿。本课用四个最常用的动画段串起一条完整时间线。

<video controls loop muted preload="metadata" src="media/lesson_anims.mp4"></video>

```rust
{{#include ../../../crates/ffanim/examples/lessons.rs:lesson_anims}}
```

## 要点

- **`Anim::create`** 描边生长（闭合填充字形要改用 `fade_in`）；
  **`transform`** 逐点变形；**`move_to`** 绝对平移；**`fade_out`** 淡出。
- **`.during(秒)`** 定时长；**`.rate(RateFunc::...)`** 定缓动——默认
  `Smooth`（先加速后减速），可换 `Linear`（匀速）、`EaseIn` 等。
- **`Anim::transform(a, b)`** 的目标 `b` 必须先 `tl.add` 注册——
  变形是逐点插值，两个形状的贝塞尔结构会自动对齐。
- 平移家族还有相对版：`shift_to`（相对位移）、`rotate_to`、`scale_to`。

## 动画词汇全景

| 意图 | 词汇 |
|---|---|
| 出场 | `create` / `fade_in` / `grow_from_center` / `spin_in` |
| 退场 | `uncreate` / `fade_out` |
| 变化 | `transform` / `move_to` / `shift_to` / `rotate_to` / `scale_to` |
| 编排 | `tl.play`（接续）/ `tl.wait`（停顿）/ `tl.group`（错峰）/ `tl.together`（同窗）/ `tl.overlap`（叠窗） |

## 试着改一改

- 给 `transform` 加 `.rate(RateFunc::Linear)`——对比缓动差异
- 在 `create` 前面加 `tl.play(Anim::fade_in(rect2)...)`（自己注册一个）——
  体验多对象并行编排
- 把所有 `during` 乘 2——整条时间线慢放

下一课请出文字与公式 →[第 3 课 · 文字与公式](lesson-text.md)
