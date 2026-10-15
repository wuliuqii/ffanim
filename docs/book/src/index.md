# ffanim 数学动画引擎

ffanim 是 Rust + Typst + Vello 的数学动画引擎：**manim 的数学语义 +
现代 Rust 工程化**。公式排版走 Typst，GPU 渲染走 Vello，时间轴是
纯函数求值——渲染速度实时率 >1x。

本书按三条路径组织，按需取用：

| 路径 | 适合谁 | 去哪 |
|---|---|---|
| **只看** | 学生 / 老师 / 科普创作者，不想碰代码 | [效果画廊](effect-gallery.md) |
| **上手写** | 想亲手跑出一条动画的人（不需要 Rust 基础） | [第一个动画](first-animation.md) |
| **查源码** | 开发者，看全部官方场景的可复现源码 | [场景画廊](basic.md) |

全部场景源码位于 [`crates/ffanim/examples/`]，是本书的唯一真相源——
「场景画廊」各页由 `ffanim-gally` 从示例源码自动生成（标题 / 场景清单 /
doc 摘要 / 代码段），新增或修改场景后运行 `just book` 即可重建。

## 30 秒上手（开发者）

```bash
git clone <repo-url> && cd ffanim
cargo run --release -p ffanim --example getting_started
```

出片 `out/getting_started.mp4`（960×540@30fps）。把 `getting_started`
换成任意场景名渲染对应场景，或省略场景名渲染该文件全部场景（≙ `manim -a`）。

## 场景寻址

每个示例文件对应一个场景族，文件内 `SCENES` 常量列出全部场景（argv 寻址）：

| 示例文件 | 场景族 |
|---|---|
| `getting_started.rs` | 入门：矩形生长 → 变圆 → 淡出 |
| `basic.rs` | 动画词汇 / 坐标系与函数图像 / 几何基元 / 相对布局 / 强调 / 语法糖 / 路径移动 / 结构匹配形变 / 布尔 / 轨迹 / 相机跟随 |
| `typography.rs` | 公式排版与变换 / 富文本 / 标注复合件 / 代码高亮 / 按词着色 |
| `assemblies.rs` | 花括号 / 定界符 / 箭头 / 数字滚动 |
| `tables.rs` | 表格 |
| `fourier.rs` | 创作示例：傅里叶级数 |
| `media.rs` | 创作示例：SVG 与图片 |
| `pip.rs` | 创作示例：画中画 |

## 本地构建本书

```bash
just book         # 生成书源（渲染缺失产物）+ mdbook build
just book-serve   # 热重载预览（http://localhost:3000）
```

构建产物在 `docs/book/book/`，纯静态，可直接部署任意静态托管。

[manim example gallery]: https://docs.manim.community/en/stable/examples.html
[`crates/ffanim/examples/`]: https://github.com/gallon/ffanim/tree/main/crates/ffanim/examples
