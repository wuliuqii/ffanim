# 效果画廊

按「**你想要什么效果**」浏览 ffanim 的能力——不需要懂 Rust，先看效果，
再按需点进源码。每类配一段代表视频，表格里是可以直接使用的动画词汇
（点到对应场景的完整源码）。

## 形状的出现与消失

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 像手绘一样「画」出形状 | `Anim::create` | [demo_anim](basic.md#demo_anim) |
| 手绘的逆过程：描边收回 | `Anim::uncreate` | [sugar_demo](basic.md#sugar_demo) |
| 淡入 / 淡出 | `fade_in` / `fade_out` | [demo_anim](basic.md#demo_anim) |
| 从下方滑入、从中心长大、旋转入场 | `fade_in_from` / `grow_from_center` / `spin_in` | [sugar_demo](basic.md#sugar_demo) |

<video controls loop muted preload="metadata" src="media/demo_anim.mp4"></video>

## 变形

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 一个形状平滑变成另一个 | `Anim::transform` | [demo_anim](basic.md#demo_anim) |
| 公式按结构匹配着变 | `match_formulas` | [formula_demo](typography.md#formula_demo) |
| 任意图形按片段匹配着变 | `match_shapes` | [shapes_demo](basic.md#shapes_demo) |

<video controls loop muted preload="metadata" src="media/shapes_demo.mp4"></video>

## 移动与路径

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 平移 / 旋转 / 缩放到目标位姿 | `shift_to` / `rotate_to` / `scale_to` / `move_to` | [primitives_demo](basic.md#primitives_demo) |
| 沿任意曲线移动（弧长匀速） | `Anim::move_along_path` | [move_demo](basic.md#move_demo) |
| 身后留下轨迹 | tracker + 双 updater（拼装） | [trace_demo](basic.md#trace_demo) |

<video controls loop muted preload="metadata" src="media/move_demo.mp4"></video>

## 强调重点

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 轻晃提醒「注意这一项」 | `tl.indicate` | [emphasize_demo](basic.md#emphasize_demo) |
| 圈框强调 | `tl.circumscribe` | [emphasize_demo](basic.md#emphasize_demo) |
| 位置闪烁 | `tl.flash` | [emphasize_demo](basic.md#emphasize_demo) |
| 光点沿曲线划过 | `tl.show_passing_flash` | [emphasize_demo](basic.md#emphasize_demo) |

<video controls loop muted preload="metadata" src="media/emphasize_demo.mp4"></video>

## 布局与标注

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 相对摆放 / 贴边 / 排队 | `next_to` / `to_edge` / `arrange` | [layout_demo](basic.md#layout_demo) |
| 框选 / 下划线 / 划叉 / 底色 | `surrounding_rect` / `underline` / `cross` / `background_rect` | [annotate_demo](typography.md#annotate_demo) |
| 花括号 + 括注 | `TypstEngine::delimiter` | [brace_demo](assemblies.md#brace_demo) |
| 指向箭头（几何 / 装配两族） | `Item::arrow` / `TypstEngine::arrow` | [arrow_demo](assemblies.md#arrow_demo) |

<video controls loop muted preload="metadata" src="media/layout_demo.mp4"></video>

## 文字与公式

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 排版数学公式（Typst 语法） | `TypstEngine::math` | [formula_demo](typography.md#formula_demo) |
| 富文本混排 | `TypstEngine::text` | [text_demo](typography.md#text_demo) |
| 按词着色 | `text_t2c` | [t2c_demo](typography.md#t2c_demo) |
| 代码块高亮 | `TypstEngine::code` | [code_demo](typography.md#code_demo) |

<video controls loop muted preload="metadata" src="media/formula_demo.mp4"></video>

## 坐标系与函数图像

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 画函数曲线 | `Axes::plot` | [sine_demo](basic.md#sine_demo) |
| 动点与图像联动（tracker 驱动） | `add_tracker` + updater | [sine_demo](basic.md#sine_demo) |
| 单位圆展开成正弦波 | 双 updater 参数化联动 | [circle_sine_demo](basic.md#circle_sine_demo) |
| 曲线下面积 / 黎曼矩形 | c2p 采样拼多边形 | [riemann_area](basic.md#riemann_area) |
| 角度标记跟随 | 弧 + 双线 + θ 标注拼装 | [angle_demo](basic.md#angle_demo) |

<video controls loop muted preload="metadata" src="media/sine_demo.mp4"></video>

## 数据可视化

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 表格（逐行动画） | `ffanim::table` | [table_demo](tables.md#table_demo) |
| 折线图 + 散点 | polyline + circle 拼装 | [line_graph](basic.md#line_graph) |
| 数字滚动 | `decimal_signal` | [decimal_demo](assemblies.md#decimal_demo) |
| 矩形随数据重绘 + 面积联动 | 双 tracker + `DecimalNumber` | [polygon_axes_demo](basic.md#polygon_axes_demo) |

<video controls loop muted preload="metadata" src="media/table_demo.mp4"></video>

## 镜头语言

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 相机推近 / 拉远（注视点+缩放） | `Camera::affine` + `transform_to` | [follow_cam_demo](basic.md#follow_cam_demo) |
| 相机跟随动点平移 | 相机 `move_along_path` 同窗同步 | [follow_cam_demo](basic.md#follow_cam_demo) |
| 画中画局部放大 | 出片层拼装（`render_frame` + 重渲染） | [pip](pip.md) |

<video controls loop muted preload="metadata" src="media/follow_cam_demo.mp4"></video>

## 进阶拼装

ffanim 的设计原则是**最小原语**：下面这些"高级效果"没有专用类，
全部由现有词汇组合而成——读懂它们是掌握引擎的最佳练习。

| 你想要 | 用什么 | 源码 |
|---|---|---|
| 布尔运算（并/交/差/异或） | `Item::boolean` | [boolean_demo](basic.md#boolean_demo) |
| 网格逐点非线性形变 | 折线采样 + tracker 逐帧重建 | [warp_grid_demo](basic.md#warp_grid_demo) |
| 傅里叶级数逼近方波 | 全流程创作示例 | [fourier](fourier.md) |
| SVG 导入与图片 | `from_svg` / `from_image` | [media](media.md) |

<video controls loop muted preload="metadata" src="media/fourier.mp4"></video>

---

看中了哪个？到[第一个动画](first-animation.md)花十分钟亲手跑起来。
