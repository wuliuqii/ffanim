# 实战拆解：傅里叶级数

[fourier.rs](fourier.md) 是第一个**真实创作**的完整作品：约 200 行 Rust，
一条 20 秒的科普短片。本页把它按创作流程拆开——读完你会发现，
它的每一幕都只用到分课教程里讲过的概念。

<video controls loop muted preload="metadata" src="media/fourier.mp4"></video>

完整源码：[fourier.md（画廊页）](fourier.md) · 全部源码在
`crates/ffanim/examples/fourier.rs`。

## 创作脚本先行

写代码前先写脚本（一两句话即可）：

> 标题 → 级数公式逐字写出 → 坐标系与方波目标登场 → 部分和 S_N 逐阶
> 升高逼近方波 → 推近看吉布斯过冲 → 结论。

对应到 ffanim 就是**五幕**，每幕 = 几个 `tl.play` + `tl.wait`。

## 幕 1 · 标题淡入

```rust
{{#include ../../../crates/ffanim/examples/fourier.rs:fourier_title}}
```

知识点：`engine.text` 排版 → `to_items()` 拆散件 → `fit_transform`
落位 → `group(..., 0.0)` 同窗淡入（步长 0 = 整体同时出场）。

## 幕 2 · 公式逐字形写出（Write 效果）

```rust
{{#include ../../../crates/ffanim/examples/fourier.rs:fourier_write}}
```

知识点：公式用 Typst math 语法（`frac(a,b)` 分数、`dots` 省略号）；
`group(..., 0.06)` 步长 0.06s——**逐字形错峰 = Write**，这就是 manim
`Write` 的全部秘密，没有专用类。

## 幕 3 · 坐标系与方波目标

```rust
{{#include ../../../crates/ffanim/examples/fourier.rs:fourier_axes}}
```

知识点：网格（`grid_item`）淡入、轴（`to_item`）描边生长分两步；
方波是自定义曲线（`Item::from_path` 折线），`c2p` 把数据坐标换成
像素——**任何"手画"的图形都能进坐标系**。

## 幕 4 · 部分和逼近 + 推近镜头

```rust
{{#include ../../../crates/ffanim/examples/fourier.rs:fourier_tracker}}
```

这是全片的灵魂，三件事：

- **tracker 承载数学量 N**：`add_tracker(1.0)` 建立变量，
  `set_value` 让它跳到 3、5、7、9。
- **updater 每帧重建曲线**：闭包里按 `v[0]`（当前 N）计算部分和
  S_N(x) 并 `axes.plot`——部分和公式就是普通的 Rust 循环。
- **相机推近**：`tl.camera` 注册相机，`transform_to` 到
  `zoom: 1.4` 的取景——关注跳变处的过冲细节。

## 幕 5 · 出片与无看片诊断

```rust
{{#include ../../../crates/ffanim/examples/fourier.rs:fourier_render}}
```

知识点：`compile()` 把构建期记录编译成求值表；`render_to_mp4`
出片；`BGM=xx.wav` 时自动分析音频、延伸片长并混流（`audio::analyze`
/ `mux`）。最后 `tools::strip`（时间轴条带图）与 `tools::onion`
（多帧叠加图）生成诊断 PNG——**不用逐帧看视频就能核对动画节奏**。

## 创作心法

| 步骤 | 用什么 |
|---|---|
| 先写脚本分幕 | 每幕 = play/wait 序列 |
| 文字公式 | `TypstEngine` + `to_items` + `fit_transform` |
| 自定义曲线 | `from_path` + `c2p`（或 `plot` 直接吃闭包） |
| 任何"随参数变化" | tracker + updater 重建 |
| 引导视线 | 相机 `transform_to` / 强调两件套 |
| 核对效果 | `strip` / `onion` 诊断图 |

下一步 → [配方 FAQ](recipes.md)
