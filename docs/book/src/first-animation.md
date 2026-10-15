# 第一个动画

花十分钟，从零跑通一条属于你的数学动画：**一个矩形被「画」出来，
平滑变形为圆，再淡出**。全程不需要看懂 Rust——跟着抄就能跑，
每一段代码只有几行。

## 先看效果

<video controls loop muted preload="metadata" src="media/getting_started.mp4"></video>

## 环境准备

只需要两步（一次性）：

```bash
# 1) 安装 Rust（若已有可跳过）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2) 拿到 ffanim 源码
git clone <repo-url> && cd ffanim
```

渲染需要系统安装 [ffmpeg](https://ffmpeg.org/download.html)（出片编码用）。

## 跑起来

```bash
cargo run --release -p ffanim --example getting_started
```

产物在 `out/getting_started.mp4`（960×540@30fps，约 2.5 秒）。恭喜——
这条命令做的事是：编译入门示例 → 逐帧渲染 → ffmpeg 编码成 MP4。

## 逐段读懂代码

完整源码在 `crates/ffanim/examples/getting_started.rs`，总共不到 50 行。
核心只涉及三个概念：

> - **Item** —— 画什么（形状、文字、公式，一切都是 Item）
> - **Timeline** —— 何时画（一条时间轴，管所有对象和动画的先后）
> - **Anim** —— 怎么变（create / transform / fade_out…都是时间轴上的动画段）

### 第 1 段：搭舞台

```rust
{{#include ../../../crates/ffanim/examples/getting_started.rs:setup}}
```

主题从 JSON 文件读入（颜色、描边宽度都不写死在代码里），创建一条
960×540、30fps 的空时间轴，背景色取自主题。

### 第 2 段：画出矩形

```rust
{{#include ../../../crates/ffanim/examples/getting_started.rs:create_item}}
```

两行核心：`tl.add(...)` 把矩形**注册**到时间轴上（得到一个句柄），
`tl.play(Anim::create(...))` 排入一段 1 秒的描边生长动画——就像手绘
一样从无到有把边框「画」出来。

### 第 3 段：变形为圆

```rust
{{#include ../../../crates/ffanim/examples/getting_started.rs:morph}}
```

先注册目标圆形，再用 `Anim::transform(square, circle)` 把矩形**逐点
变形**为圆——两者的轮廓都是 4 段三次贝塞尔曲线，引擎自动做逐点插值，
所以过渡是真正意义的形状渐变，而不是交叉淡化。

### 第 4 段：收尾并出片

```rust
{{#include ../../../crates/ffanim/examples/getting_started.rs:finish}}
```

`fade_out` 淡出，然后 `tl.compile()` 把构建期记录的动作编译成
求值表，`render_to_mp4` 逐帧渲染并交给 ffmpeg 编码。

## 试着改一改

不用看任何文档，直接改数字重跑：

- 把两处 `during(1.0)` 改成 `during(2.0)`——动画变慢
- 把 `Rect::new(370.0, 165.0, 590.0, 375.0)` 改小——矩形变小
- 把 `Anim::create(...)` 换成 `Anim::fade_in(...)`——换成淡入出场
- 在 `fade_out` 之前加一行 `tl.wait(0.5)`——结尾停半秒

改坏了大不了还原，渲染只要几秒。

## 下一步

- 想快速能做什么 → [效果画廊](effect-gallery.md)
- 想系统学 → [分课教程](lesson-shapes.md)六课，每课 30 行代码
- 想看真实场景 → [场景画廊](basic.md)的 `demo_anim` 逐场景读源码
