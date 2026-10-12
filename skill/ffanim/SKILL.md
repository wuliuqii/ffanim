---
name: ffanim
description: 用 Rust 写数学动画（manim 替代品）：两阶段时间轴 API、动画词汇表、agent 无看片验证循环（timeline/inspect/strip/onion/snapshot）。当用户要求制作数学动画、教学视频、公式动画，或询问 ffanim 用法时使用。
---

# ffanim — 用 Rust 写数学动画

单二进制数学动画引擎：Rust + Typst + Vello，无 TeX/Node/Python 依赖。
核心理念：**作者（或 agent）无法看视频**——一切验证通过 CLI 把视频变成 PNG/文本/数字。

## 心智模型：两阶段时间轴

1. **构建期**：`Timeline::play()` 顺序记录动画动作（不立即求值）；
2. **求值期**：`compile()` 得不可变 `CompiledTimeline`，`render_frame(tl, frame)` 是确定性纯函数——同输入恒同输出，跳帧求值天然支持。

```rust
use ffanim::kurbo::{Circle, Rect};
use ffanim::{Anim, Color, Item, Timeline};

let (w, h) = (960u32, 540u32);
let mut tl = Timeline::new(w, h, 30, Color::new(14, 17, 22, 255));

let cir = tl.add(
    Item::circle(Circle::new((480.0, 300.0), 100.0))
        .with_stroke(Color::new(120, 220, 232, 255))
        .with_stroke_width(3.0),
);
tl.play(Anim::create(cir).during(1.2));
tl.wait(0.5);

ffanim::render_to_mp4(&tl.compile(), "out/demo.mp4")?;
```

## 对象与动画词汇

- `tl.add(item) -> ItemId`：注册对象（初始状态）。动画不修改原对象，渲染期按调度表插值。
- `tl.add_tracker(v0) -> TrackerId` / `Anim::set_value(tracker, to)`：值动画（与其他动画同表调度）。
- `tl.add_updater(&[deps], move |v| Item)`：注册信号节点，渲染期按 tracker 值**逐帧重建**形状（如函数图像随参数变化）。
- 基元构造器：`Item::rect / circle / regular_polygon / from_path / from_image` +
  标注五件套 `Item::line(a, b)` / `arrow(a, b)`（线+闭合三角头，需同色 fill+stroke）/
  `dashed_line(a, b, dash, gap)` / `arc(center, r, 起角°, 止角°)`（视觉逆时针）/
  `ellipse(center, rx, ry)`；`item.bbox() -> Rect` 查包围盒（布局/对齐用）。
- 相对布局（构建期纯操作，manim `next_to`/`to_edge`/`arrange` 对应）：
  `item.next_to(&target, Direction::Right, buff)`（邻接 + 另一轴中心对齐）/
  `item.to_edge(Direction::Up, buff, (w, h))`（贴画布边，另一轴居中）/
  `arrange(&mut items, Direction::Down, buff) -> Rect`（依次排列，返回总包围盒，可再整体平移）。
  布局分支带 `log::info` 埋点（RUST_LOG=info 可查输入 bbox/平移量/结果 bbox）。
- 标注复合件（manim SurroundingRectangle/Underline/Cross/BackgroundRect 对应，P1-7）：
  `Item::surrounding_rect(&target, buff)`（圈选框）/ `Item::underline(&target, buff)`（底线）/
  `Item::cross(&target, buff)`（对角叉）/ `Item::background_rect(&target, buff)`（背景板，
  **注册须先于目标**——z 序 = 注册顺序）。样式链式自定（半透明用 `Color { a, ..theme色 }`）；
  公式等排版散件先 `Item::group(items)` 拼组再标注（**拼组仅作几何用途，不注册**）。
  **标注基于注册基线 bbox：目标有位姿动画时，标注需挂相同 shift_to 同步**（基线不随动画变）。
  四构造器 + group 带 log::info 埋点（RUST_LOG=info 查目标 bbox/外扩框/底线坐标）。
- 组编排糖（manim Together/Succession/LaggedStart/VGroup/save_state·restore 对应）：
  `tl.together([...])`（多动画同窗对齐，游标取最长 dur——**规避 overlap 各段 during 必须一致的坑**）/
  `tl.succession([...])`（逐段接续播）/
  `tl.add_group(vec![Item]) -> Vec<ItemId>`（VGroup 批量注册）/
  `tl.save_state(id) -> Item` + `tl.restore(id, &snap) -> ItemId`（快照 + 完整 morph 回快照；
  **restore 返回新句柄，morph 后旧句柄退场，后续动画用返回值**）。
  together/succession/restore 带 log::info 埋点（RUST_LOG=info 查同窗/接续时序）。
- 强调四件套（构建期编排，manim Indicate/Circumscribe/Flash/ShowPassingFlash 对应）：
  `tl.indicate(id, factor, dur)`（绕 bbox 中心放大 factor 再复原——"注意这一项"）/
  `tl.circumscribe(id, dur, buff, color)`（bbox 外扩 buff 的框淡入淡出圈选）/
  `tl.flash(at, dur, radius, color)`（at 处扩散圆环一闪，指向空白处）/
  `tl.show_passing_flash(id, t0, t1, dur, color)`（沿对象路径 [t0,t1] 段局部高亮描边，
  需 Shape::Path）。四者埋点齐全（RUST_LOG=info 查峰值坐标/框范围/时序窗口）。

| 动画 | 效果 |
|---|---|
| `Anim::create(id)` | 描边生长（沿路径逐段显现） |
| `Anim::uncreate(id)` | create 镜像：描边收回 + 填充淡出，段后隐藏（manim Uncreate） |
| `Anim::fade_in(id)` / `fade_out` | 透明度渐变 |
| `Anim::fade_in_from(id, dir, dist)` | 方向滑入（Direction::Up/Down/Left/Right 偏移 dist） |
| `Anim::grow_from_center(id)` / `grow_from_edge(id, dir, dist)` | 从中心缩放 0→1 / 从边缘滑入+展开 |
| `Anim::spin_in(id, 弧度)` | 旋转进入（缩放 0→1 + 余量角收正，正 = 视觉逆时针） |
| `Anim::transform(from, to)` | 形变（需同为路径，逐点映射） |
| `Anim::shift_to(id, dx, dy)` | 相对平移到（manim `.animate.shift`） |
| `Anim::move_to(id, 点)` | 平移至 bbox 中心落在目标点 |
| `Anim::rotate_to(id, 弧度)` | 绕 bbox 中心旋转（正 = **视觉逆时针**） |
| `Anim::scale_to(id, 系数)` | 绕 bbox 中心缩放 |
| `Anim::move_along_path(id, &path)` | 沿路径移动（manim MoveAlongPath）：弧长匀速采样 + 缓动作用于进度，位移 = 采样点 − 路径起点（**起点对齐**，与对象初始位置无关），段后驻停路径终点 |
| `Anim::set_value(tracker, to)` | tracker 值动画 |
| `.during(s)` | 时长（秒）；`.shift(dx, dy)` 位移修饰；`RateFunc` 缓动（Linear/Smooth/EaseIn/EaseOut/ThereAndBack） |

> 位姿四件套（shift_to/move_to/rotate_to/scale_to）在 `play()` 时按构建期基线解析为
> 绝对 TransformTo，可连续串接（后段基于前段终点）。

### ⚠️ `tl.overlap(d)` 语义（最常见坑）

下一动画的 start = **游标回退 d 秒**（制造重叠）。两个推论：

- 逐项错开（第 i 项延迟 step）：`tl.overlap(during - step)`（不是 step！）；
- 多动画对齐同一窗口：各动画 during 必须一致，回退自身 during 才回到共同起点。

## 坐标系与公式

```rust
use ffanim::Axes;
let axes = Axes::new((-3.0, 3.0), (-1.2, 1.2))   // 数据范围
    .with_size(840.0, 400.0)                      // 像素尺寸
    .with_center(480.0, 270.0);                   // 画布居中
axes.grid_item(fg.with_alpha(40), 1.0)            // NumberPlane 网格
axes.to_item(&style)                              // 轴 + 刻度
axes.plot(|x| x * x, (-3.0, 3.0))                 // 函数图像（Catmull-Rom 平滑）
```

Typst 公式（依赖 `ffanim-typst`，公式排版有 LRU 缓存）：
`TypstEngine::formula("x^2 + y^2 = 0", &opts)` → 逐字形动画、`TransformMatchingTex` 结构匹配形变。
注意：符号 `integral`（不是 `int`）。

代码块高亮（P1-13，manim Code 对应）：
`TypstEngine::code(body, "rust", &CodeOptions::new(13.0))` → syntect 多色高亮的逐字形 `Formula`
（`to_items`/`fit_transform` 与公式原样复用）。主题 `CodeTheme::Dark/Light`（内嵌 GitHub 色板
tmTheme，token 色属排版引擎资产不走场景主题 JSON）；`lang` 空串 = 纯文本，正文含 ``` 时围栏自动升级。
**前景色语义：Typst raw 高亮只给命中 token 设 fill，普通文本继承外层 `#set text(fill:)`——
引擎已自动填入主题前景色（`CodeTheme::foreground_hex()`），画布配色可据此对齐**。
code 带埋点（RUST_LOG=info 查语言/行数/主题 → 宽高/片段数/着色种数/耗时）。

## Agent 验证循环（无人工看片）

写完场景后按序执行，全部机器可判定：

```bash
ffanim timeline --scene my_scene        # ① 调度表：动画区间/时长是否符合意图
ffanim inspect --scene my_scene         # ② 采样诊断：可见项数/越界项（exit code 2 = 异常）
ffanim render --scene my_scene --out out/v.mp4   # ③ 出片
ffanim strip --scene my_scene           # ④ 接触表：网格采样帧，看全局节奏与构图
ffanim onion --scene my_scene --frame N # ⑤ 洋葱皮：运动轨迹残影
ffanim snapshot --scene my_scene --frame N       # ⑥ 基准 diff（回归测试）
```

- 都支持 `--json`（机器解析）。
- 音轨：`ffanim audio --input x.wav`（时长/RMS/削波/静音段）；`render --audio x.wav` 混流且片长延伸到音频长度。
- 预览器：`ffanim preview --scene x`（交互确认用，非 agent 路径）。

## 数据驱动约束

颜色/主题从 JSON 加载（`Theme::from_path`），代码零硬编码颜色。
`ffanim new my-anim [--template minimal|coords] [--dev]` 生成脚手架（含 themes/default.json）。

## 常见坑

1. 画布 y 轴向下（屏幕坐标）；`Axes` 已在 c2p 内处理翻转。
2. `overlap` 回退语义（见上）——调度不符合预期先查 `timeline` 输出。
3. 输出需要 ffmpeg 在 PATH；1080p30 渲染耗时 ≤ 视频时长。
4. vello_cpu 画布尺寸上限 65535px；宽高必须为偶数（yuv420p）。
