# ffanim 示例画廊

ffanim 是 Rust + Typst + Vello 的数学动画引擎。本书记录全部官方示例：
**每个场景一段视频 + 一段可复现的源码**，形态对齐 [manim example gallery]。

全部场景源码位于 [`crates/ffanim/examples/`]，是本书的唯一真相源——
本章其余页面由 `ffanim-gally` 从示例源码自动生成（标题 / 场景清单 / doc
摘要 / 代码段），新增或修改场景后运行 `just book` 即可重建。

## 30 秒上手

```bash
git clone <repo-url> && cd ffanim
cargo run --release -p ffanim --example basic -- demo_anim
```

出片 `out/demo_anim.mp4`（960×540@30fps）。把 `demo_anim` 换成任意场景名，
或省略场景名渲染该文件全部场景（≙ `manim -a`）。

## 场景寻址

每个示例文件对应一个场景族，文件内 `SCENES` 常量列出全部场景（argv 寻址）：

| 示例文件 | 场景族 |
|---|---|
| `basic.rs` | 动画词汇 / 坐标系与函数图像 / 几何基元 / 相对布局 / 强调 / 语法糖 / 路径移动 / 结构匹配形变 |
| `typography.rs` | 公式排版与变换 / 富文本 / 标注复合件 / 代码高亮 / 按词着色 |
| `assemblies.rs` | 花括号 / 定界符 / 箭头 / 数字滚动 |
| `tables.rs` | 表格 |
| `fourier.rs` | 创作示例：傅里叶级数 |
| `media.rs` | 创作示例：SVG 与图片 |

## 本地构建本书

```bash
just book         # 生成书源（渲染缺失产物）+ mdbook build
just book-serve   # 热重载预览（http://localhost:3000）
```

构建产物在 `docs/book/book/`，纯静态，可直接部署任意静态托管。

[manim example gallery]: https://docs.manim.community/en/stable/examples.html
[`crates/ffanim/examples/`]: https://github.com/gallon/ffanim/tree/main/crates/ffanim/examples
