# ffanim

用 Rust 写数学动画 —— **manim 的数学语义 + 现代 Rust 工程化**（GPU 渲染、类型安全、agent 友好）。

📖 **[在线示例书](https://wuliuqii.github.io/ffanim/)** —— 全部 44 个场景：视频 + 可复现源码，GitHub Pages 自动部署。

![ffanim 演示](docs/demo.gif)

## 特性

- **时间轴两阶段模型**：`play()` 构建期记录 → 编译为静态调度表 → 渲染期纯函数查表求值。无运行时闭包，帧段可并行。
- **单一 Item 表示**：无 Mobject/VMobject 分裂；变换挂 Item 不烘焙进点阵。
- **Typst 公式内嵌**：库级嵌入排版（无 LaTeX 外部进程）+ LRU 缓存 + 公式结构匹配（TransformMatchingTex）。
- **双后端渲染**：vello GPU 为主 + vello_cpu 兜底，与 mp4 出片像素级一致。
- **Agent 验证循环**：timeline 检查 / inspect 采样 / strip 接触表 / onion 洋葱皮 / snapshot 像素回归。
- **数据驱动**：主题与场景参数全部来自 JSON，代码零硬编码颜色。

## 30 秒上手

```rust
use ffanim::{Anim, Color, Item, Timeline};

let mut tl = Timeline::new(960, 540, 30, Color::new(14, 17, 22, 255));
let cir = tl.add(Item::circle(ffanim::kurbo::Circle::new((480.0, 270.0), 80.0)));
tl.play(Anim::create(cir).during(1.0));
ffanim::render_to_mp4(&tl.compile(), "out/demo.mp4").unwrap();
```

或直接跑官方示例（argv 寻址，缺省全渲染）：

```sh
just demo                      # demo_anim 场景出片（三角形生长 / 圆滑入 / 形变 / 标题）
cargo run -p ffanim --example basic -- sine_demo   # 任意场景，或省略场景名全渲染
just fourier                   # 傅里叶级数创作示例（Typst 公式 + BGM 混流）
```

全部 16 个示例文件（44 个场景）的**视频 + 源码**见[在线示例书](https://wuliuqii.github.io/ffanim/)（或 `just book` 本地构建）。

## CLI 工具箱

| 命令 | 用途 |
|---|---|
| `ffanim new my_anim` | 脚手架：空目录 → 最小可跑工程（`--template` 选模板） |
| `ffanim audio -i <文件>` | 音频诊断：时长/电平/削波/静音段（`--json` 机器可读） |
| `ffanim info` | 加载并展示主题摘要（验证数据驱动管线） |
| `ffanim diff <公式A> <公式B>` | 公式片段匹配预览（TransformMatchingTex 调度计划） |

> 场景出片/预览直接跑 [examples](crates/ffanim/examples/)（对齐 manim
> example_scenes 形态）；agent 逐帧检查用 `--json` 打印调度表。

## 与 manim 的差异

| 维度 | manim | ffanim |
|---|---|---|
| 语言 | Python（运行时解释） | Rust（编译期类型检查） |
| 公式 | LaTeX 外部进程 | Typst 库内嵌，进程内排版 |
| 时间轴 | 运行时闭包逐帧回调 | 编译期静态调度表，渲染纯函数 |
| 渲染 | cairo / OpenGL 混合 | vello GPU 为主 + CPU 兜底 |
| 工作流 | 人眼看片为主 | 内置 agent 验证循环（timeline→strip→snapshot） |

详细调研见 [docs/01-调研对比报告.md](docs/01-调研对比报告.md)。

## 文档

- **[在线示例书](https://wuliuqii.github.io/ffanim/)** —— 全部 16 个示例文件（44 个场景）：视频 + 可复现源码（`just book` 本地构建）
- [架构设计](docs/02-架构设计.md) —— 分层、时间轴模型、渲染管线
- [路线图与风险](docs/03-路线图与风险.md) —— M0~M10 里程碑
- [项目优化规划](docs/04-项目优化规划.md) —— v0.2.0 起的演进计划
- [更新日志](CHANGELOG.md)

## 开发

```sh
just ci        # 质量门禁：fmt + clippy(-D warnings) + test
just ci-gpu    # GPU 特性编译验证
just doc       # rustdoc 零警告门禁
just bench     # 性能基准（criterion）
```

MSRV 跟随最新 stable（当前 1.95）。许可 [MIT](LICENSE)。
