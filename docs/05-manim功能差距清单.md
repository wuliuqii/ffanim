# 05 — ffanim 与 manim 功能差距清单

> 对比基线：ffanim v0.3.0 + M9.1/M9.2（2026-10-12）vs manim CE v0.19。
> 视角：**2D 数学教学视频**（ffanim 的目标场景）；3D、交互式 Notebook 等超出目标的能力只登记不追平。
> 优先级定义（按业务重要性 / 用户影响排序）：
> - **P0**：教学视频高频刚需，缺失直接阻塞创作或迫使用户手写底层代码绕行；
> - **P1**：常用能力，有可接受的 workaround 或使用频率中等；
> - **P2**：低频 / 大工程 / 与 ffanim 定位冲突，暂缓。
> 工作量：S（≤1 天）/ M（2~3 天）/ L（一周级）。

## 1. 总体结论

| 领域 | manim | ffanim | 覆盖度 | 结论 |
|---|---|---|---|---|
| 时间轴模型 | Scene 逐帧回调 + 动画对象 | 两阶段（play 记录 → 纯函数查表） | ✅ 超越 | 跳帧求值/确定性/无运行时闭包，manim 不具备 |
| 缓动 | rate_functions 家族 | Linear/Smooth/EaseIn/EaseOut/ThereAndBack | ≈90% | 缺 swoope/smoothstep 等冷门曲线（P2） |
| 公式 | LaTeX(MathTex) | Typst（LRU 缓存） | ✅ 等价替代 | 逐字形 pieces + source_range，无 TeX 依赖是优势 |
| 公式结构变换 | TransformMatchingTex | match_formulas + move 配对（M9.1） | ✅ 对齐 | move 检测与 manim 同级 |
| 富文本 | Text(Pango) | Typst markup 混排（M8.2） | ✅ 等价 | 缺 t2c 按词着色的显式 API（P1，markup 可绕） |
| ValueTracker/updater | ValueTracker + updater 闭包 | add_tracker + add_updater（信号节点） | ✅ 对齐 | ffanim 渲染期纯函数重建，更可测 |
| 组动画 | AnimationGroup/LaggedStart | group/lagged_start（M8.3） | ✅ 对齐 | 缺 Succession/Together 语义糖（P1 薄糖） |
| 相机 | MovingCameraScene | Camera 即 Item + transform_to（M8.1） | ✅ 对齐 | 缺 3D 相机（P2） |
| 几何基元 | 30+ 类 | circle/rect/regular_polygon/from_path | **≈25%** | **P0 缺口，见 §2.1** |
| 相对布局 | next_to/align_to/to_edge/arrange | 无 | **0%** | **P0 缺口，见 §2.2** |
| 强调动画 | Indicate/Circumscribe/Flash/… | 无 | **0%** | **P0 缺口，见 §2.3** |
| 动作变换 | .animate.shift()/rotate()/scale() | TransformTo(Affine) 覆盖但无便捷构造 | ≈50% | **P0 薄糖，见 §2.4** |
| SVG/图片 | SVGMobject/ImageMobject | from_svg/from_image（M8.4） | ✅ 对齐 | — |
| 音频 | add_sound（时间轴对齐） | analyze/mux/render --audio | ≈70% | 缺时间轴内嵌音效（P2） |
| 片段导出 | -n 起止动画号 / sections | --section 秒区间（M9.2） | ✅ 对齐 | manim 按动画号，ffanim 按秒（各自合理） |
| Agent 验证 | 无（需人工看片） | timeline/inspect/strip/onion/snapshot | ✅ 超越 | ffanim 核心差异化优势 |
| 依赖 | Python + LaTeX + Pango/ffmpeg | 单二进制 + ffmpeg | ✅ 超越 | — |
| 3D | ThreeDScene/Surface 全家桶 | 无 | 0% | P2，超出 2D 目标场景 |
| 缓存 | 动画哈希缓存 | 无 | — | ffanim 渲染快且纯函数，收益低（P2 不追） |

## 2. 差距清单（按优先级）

### P0 —— 实施中

| # | 功能 | manim 对应 | ffanim 现状 | 用户影响 | 工作量 |
|---|---|---|---|---|---|
| P0-1 | **标注基元**：线段/箭头/虚线/圆弧/椭圆 | Line/Arrow/DashedLine/Arc/Ellipse | ✅ 已落地（M9.3）：`Item::line/arrow/dashed_line/arc/ellipse` + `bbox()`，primitives_demo 场景验收 | 几何图示、指向标注、角度标记、轨迹线——教学视频的"第二高频"元素画不出来 | S |
| P0-2 | **相对布局**：bbox 查询 + next_to/to_edge/arrange | mobject.bbox、`next_to`、`to_edge`、`arrange` | ✅ 已落地（M9.4）：`Item::next_to/to_edge` + `arrange`（构建期纯操作，核心分支 log::info 埋点），layout_demo 场景验收 | agent 无预览，排版类场景（公式组、步骤列表、标注跟随）只能 inspect 试错，效率极低 | M |
| P0-3 | **强调动画**：闪现强调/框选强调/经过闪烁 | Indicate/Circumscribe/Flash/ShowPassingFlash | ✅ 已落地（M9.5）：`tl.indicate/circumscribe/flash/show_passing_flash`（构建期编排展开既有词汇，求值零改动，log::info 埋点齐全），emphasize_demo 场景验收 | 教学节奏词缺失（"注意这一项"），每个场景都用 | S~M |
| P0-4 | **动作变换糖**：shift_to/rotate_to/scale_to/move_to | `.animate.shift()`/`rotate()`/`scale()`/`move_to()` | ✅ 已落地（M9.3）：`Anim::shift_to/move_to/rotate_to/scale_to`（构建期 Pose 解析为绝对 TransformTo） | 每个位移/旋转场景都要写矩阵运算，繁琐易错；薄语法糖不破架构 | S |

### P1 —— 排队

| # | 功能 | manim 对应 | 现状 / 说明 | 工作量 |
|---|---|---|---|---|
| P1-1 | Uncreate / Write 反向（描边收尾） | Uncreate/Write | ✅ 已落地（M9.7）：`Anim::uncreate`（create 镜像：描边收回 + 填充淡出，段后隐藏），sugar_demo 场景验收 | S |
| P1-2 | 组合调度糖：Succession / Together | Succession/Together | ✅ 已落地（M9.7）：`tl.together`（同窗对齐游标取最长 dur）/ `tl.succession`（逐段接续），埋点齐全 | S |
| P1-3 | FadeIn 方向家族 / GrowFromCenter / SpinIn | GrowFromEdge/Center… | ✅ 已落地（M9.7）：FadeIn 参数化 `{shift, scale_from, spin}`，`fade_in_from/grow_from_center/grow_from_edge/spin_in` 四构造器 | S |
| P1-4 | MoveAlongPath 沿路径移动 | MoveAlongPath | 需逐帧路径采样求值（TransformTo 不够），中等 | M |
| P1-5 | 公式以外的结构匹配形变 | TransformMatchingShapes | match_formulas 基座已有（M9.1 diff 升级），扩展到任意路径文本缺失（SVG 字形无 source 文本） | M |
| P1-6 | 数字滚动 DecimalNumber | DecimalNumber + tracker | tracker+updater+Typst 组合已可行，缺"每帧改文本"的示范与便利封装（Typst LRU 缓存使逐帧排版可行） | S~M |
| P1-7 | 标注复合件：SurroundingRectangle/Underline/Cross/BackgroundRect | 同名 | ✅ 已落地（M9.8）：`Item::surrounding_rect/underline/cross/background_rect`（基于注册基线 bbox + `Item::group` 拼组标注），annotate_demo 场景验收 | S |
| P1-8 | Brace 花括号 + 括注文本 | Brace | 图形 brace 需形状生成（Typst 侧可排 brace 字符过渡） | M |
| P1-9 | GIF / WebM 输出 + 画质预设 | -ql/-qm/-qh/-qk | M9 已列（GIF 循环、WebM 编码参数）， EncodeOptions 质量预设 | M |
| P1-10 | VGroup 语义糖：批量注册 + 整体动画 | VGroup | ✅ 已落地（M9.7）：`tl.add_group(Vec<Item>) -> Vec<ItemId>`（注册侧；组级动画配 `tl.group`/`together`） | S |
| P1-11 | save_state / restore | mobject.save_state() | ✅ 已落地（M9.7）：`tl.save_state(id) -> Item` + `tl.restore(id, &snap) -> ItemId`（快照 + 完整 morph，返回新句柄） | S |
| P1-12 | 按词/按段着色 t2c | Text(t2c=…) | Typst markup 手写可绕（`#text(fill: …)`) | S |
| P1-13 | 代码块高亮 | Code | ✅ 已落地（M9.9）：`TypstEngine::code(body, lang, &CodeOptions)`（Typst raw + syntect 高亮，内嵌 Dark/Light tmTheme，普通文本前景色显式填入 `foreground_hex`），code_demo 场景验收 | M |
| P1-14 | 表格 | Table/MathTable | 手拼 grid_item + 文本可绕 | M |

### P2 —— 暂缓 / 不追

| 功能 | 说明 |
|---|---|
| 3D 全家桶（ThreeDAxes/Surface/3D 相机） | 超出 2D 教学目标；工程量 L；vello 3D 需自研投影 |
| VectorField / StreamLines | 高数进阶场景，需求低频 |
| BarChart / Heatmap / PolarPlane / ComplexPlane | 特定领域图示，可由基元+布局组合 |
| 动画哈希缓存 | ffanim 渲染实时率 >1x，重复渲染代价低；snapshot 基线已覆盖回归需求 |
| add_sound 时间轴内嵌音效 | --audio 混流已覆盖 95% 场景（BGM/旁白），逐事件音效低频 |
| LaTeX 原生 | Typst 已完全替代，不追 |
| `.animate` 链式语法 | Rust 无 monkey-patch；P0-4 命名构造器承担同等意图 |
| Notebook/交互式使用 | 定位差异，不追 |
| 冷门 rate_functions（swoope 等） | Smooth 覆盖大部分，出现需求再加 |

## 3. 实施顺序（本清单的执行计划）

1. **P0-1 标注基元**（S）→ 2. **P0-4 动作变换糖**（S，与 P0-1 同批可出一个 parity 示例）→
3. **P0-2 相对布局**（M，依赖 P0-1 的 bbox 场景验证）→ 4. **P0-3 强调动画**（S~M，复合已落地词汇）→
5. P1 从薄糖开始（P1-1/2/3/10/11 同批），再推进 M 级（P1-4/5/6/9）。

每个功能项沿用项目惯例：**验收示例代码倒推 API + 单测 + scenes.rs parity 场景 + 门禁全绿 + CHANGELOG 随项记录**。
