# 词汇速查

ffanim 门面全部公开 API（与 `scripts/api_coverage.sh` 门禁同步，
共 56 项）。按用途分组，每项一句话；「→」指向讲解它的课程或示例。

## 时间轴与动画

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `Timeline` | 时间轴：`add` 注册对象、`play/wait` 排节目单、`group/together/overlap` 编排 | [第 2 课](lesson-anims.md) |
| `CompiledTimeline` | `compile()` 产物：求值表，渲染期纯函数查表 | [实战拆解](fourier-walkthrough.md) |
| `Anim` | 动画段：`create/transform/move_to/fade_out/…` × `.during/.rate` | [第 2 课](lesson-anims.md) |
| `AnimKind` | 动画种类枚举（`describe()` 输出可读名，诊断用） | — |
| `RateFunc` | 缓动曲线：`Smooth`（默认）/`Linear`/`EaseIn…` | [第 2 课](lesson-anims.md) |
| `PoseDelta` | 相对位姿描述（`shift_to/rotate_to/scale_to` 的内部解析） | [primitives_demo](basic.md#primitives_demo) |
| `ItemId` | 对象句柄：`tl.add` 返回，动画按句柄寻址 | [第 1 课](lesson-shapes.md) |
| `Direction` | 方向枚举（贴边/排队/定界共用） | [第 5 课](lesson-layout.md) |
| `arrange` | 构建期排队布局，返回总包围盒 | [第 5 课](lesson-layout.md) |

## 对象与几何

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `Item` | 万物皆 Item：形状/文本/公式统一表示，样式挂 Item | [第 1 课](lesson-shapes.md) |
| `Style` | 样式（填充/描边/宽度），主题驱动 | [第 1 课](lesson-shapes.md) |
| `Color` | RGBA 颜色：`with_alpha` 调透明、`lerp` 派生 | [t2c_demo](typography.md#t2c_demo) |
| `Theme` | 主题：从 JSON 读色板与描边宽度（代码零硬编码色） | [第一个动画](first-animation.md) |
| `Axes` | 坐标系：`plot` 画函数、`c2p` 数据→像素、`grid_item/to_item` | [第 4 课](lesson-axes.md) |
| `Camera` | 相机（注视点/缩放/旋转），`affine(w,h)` 编译为仿射 | [第 6 课](lesson-camera.md) |
| `BoolOp` | 布尔四则 `Intersection/Union/Xor/Difference`（配 `Item::boolean`） | [配方：布尔](recipes.md) |
| `kurbo` | kurbo 几何库 re-export（`BezPath/Circle/Rect/Affine/Point/Vec2`） | [第 1 课](lesson-shapes.md) |

## 文字与公式

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `TypstEngine` | Typst 排版引擎：`formula/text/code/arrow/delimiter/brace/decimal_signal` | [第 3 课](lesson-text.md) |
| `Formula` | 排版产物：`to_items` 拆散件、`fit_transform/center_transform` 落位 | [第 3 课](lesson-text.md) |
| `FormulaOptions` | 公式选项（字号/颜色） | [第 3 课](lesson-text.md) |
| `TextOptions` | 文本选项（字号/颜色/限宽/行距） | [text_demo](typography.md#text_demo) |
| `CodeOptions` / `CodeTheme` | 代码块选项与内嵌高亮主题 | [code_demo](typography.md#code_demo) |
| `Delimiter` | 定界符种类（Typst MATH stretch 装配） | [delimiter_demo](assemblies.md#delimiter_demo) |
| `find_cjk_font` | 系统 CJK 字体查找（中文文本必注入） | [第 3 课](lesson-text.md) |
| `match_formulas` / `MatchPlan` | 公式结构匹配 → 逐字形形变计划 | [formula_demo](typography.md#formula_demo) |
| `match_shapes` | 任意图形按片段匹配形变 | [shapes_demo](basic.md#shapes_demo) |

## 表格与数字

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `table` / `TableSpec` / `TableOptions` | 表格构建（行/列/单元格样式） | [table_demo](tables.md#table_demo) |
| `DecimalNumberOptions` | 数字滚动选项（配 `decimal_signal`） | [decimal_demo](assemblies.md#decimal_demo) |

## 媒体导入

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `from_svg_file` / `from_svg_str` | SVG → Item（矢量导入） | [media](media.md) |
| `ImageAsset` | 位图资产 | [media](media.md) |

## 渲染与输出

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `render_to_mp4` | 整段出片（ffmpeg rawvideo 管道） | [第一个动画](first-animation.md) |
| `render_to_mp4_frames` | 指定帧区间出片 | — |
| `render_frame` | 取单帧 `DrawList`（出片层拼装的入口） | [配方：画中画](recipes.md) |
| `DrawList` | 一帧的绘制清单（可克隆改写后重渲染） | [配方：画中画](recipes.md) |
| `sample_frames` | 全片均匀采样帧号（诊断图取帧） | [实战拆解](fourier-walkthrough.md) |
| `downsample` | 帧位图降采样缩放 | — |
| `VERSION` | 项目版本常量 | — |

## 诊断与音频

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `strip` / `StripReport` | 时间轴条带图（一眼核对节奏） | [实战拆解](fourier-walkthrough.md) |
| `onion` / `OnionReport` | 多帧洋葱叠加图（核对运动轨迹） | [实战拆解](fourier-walkthrough.md) |
| `analyze` / `AudioReport` | 音频分析（时长/RMS，BGM 自动延片） | [实战拆解](fourier-walkthrough.md) |
| `mux` | 音视频混流 | [实战拆解](fourier-walkthrough.md) |

## 错误处理

| 词汇 | 一句话 | 讲解 |
|---|---|---|
| `Result` / `Error` | 统一错误类型（裸 `Result` 需 `use ffanim::{Result}` 防解析到 std） | — |

---

相关子模块：`typst`（排版细节）、`audio`、`tools`。
API 完整签名见 rustdoc：`cargo doc -p ffanim --open`。
