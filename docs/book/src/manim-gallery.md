# 附录：与 manim gallery 对照

以 [manim 官方 Example Gallery]（`docs/source/examples.rst`，5 大节 20 例）
为对照表逐项核对。结论先行：**20 例中 13 例已有对应示例/能力，4 例可用
现有词汇拼出，3 例存在真实能力缺口**（下表 🟡/❌），3D 类按定位不追。

## 逐项对照

| manim gallery 示例 | 内容 | ffanim 对应 | 状态 |
|---|---|---|---|
| ManimCELogo | 填充形状拼组 + 自定背景色 | `with_fill` + `Theme::background` | ✅ |
| BraceAnnotation | 花括号 + 括注 | `brace_demo`（含方向化花括号） | ✅ |
| VectorArrow | 箭头 + 坐标平面 + 标注 | `arrow_demo` + `Coords::grid_item` | ✅ |
| GradientImageFromArray | 位图导入 + 框选 | `media.rs`（`from_image` + surrounding_rect） | ✅ |
| BooleanOperations | 椭圆并/交/差布尔运算 | 无（vello 无布尔路径） | ❌ 缺口 |
| PointMovingOnShapes | 点沿圆周移动 + 绕点旋转 | `move_demo`（move_along_path）+ `Anim::rotate` | ✅ |
| MovingAround | shift/填充/scale/rotate 动作糖 | `Anim::shift_to/rotate_to/scale_to`（M9.3） | ✅ |
| MovingAngle | 两线夹角弧 + θ 随 tracker | `angle_demo`（弧+双线+θ 标注拼装，角平分线跟随） | ✅ |
| MovingDots | 双 tracker 分别驱动 x/y | `add_tracker` + 信号 updater（M8.3） | ✅ |
| MovingGroupToDestination | 组整体对齐目标点 | `move_to` + group | ✅ |
| MovingFrameBox | 公式框选强调 | `annotate_demo`（circumscribe/surrounding_rect） | ✅ |
| RotationUpdater | dt 自转 updater | 信号节点（求值期纯函数重建，语义等价） | ✅ |
| PointWithTrace | 点运动轨迹拖尾 | 逐帧拼 path 可模拟，无现成 trace 件 | 🟡 可拼 |
| SinAndCosFunctionPlot | Axes.plot 双曲线 + 竖线标注 | `sine_demo`（`Coords::plot` + 轴标注） | ✅ |
| ArgMinExample | 函数图像 + 动点跟随 | `Coords::c2p` + tracker（sine_demo 同族） | ✅ |
| GraphAreaPlot | 曲线下面积填充 + 黎曼矩形 | `riemann_area`（c2p 采样拼多边形 + tracker 收敛） | ✅ |
| PolygonOnAxes | 多边形随 tracker 重绘 | polygon + tracker 拼组 | 🟡 可拼 |
| HeatDiagramPlot | 折线/散点图 `plot_line_graph` | `line_graph`（polyline + scatter 拼装） | ✅ |
| FollowingGraphCamera | 相机跟随曲线 | Camera 即 Item + `transform_to`（M8.1） | 🟡 可拼 |
| OpeningManim | 公式 Write + 网格非线性形变 | 公式族 ✅；`apply_function` 逐点形变 ❌ | 🟡 半覆盖 |
| SineCurveUnitCircle | 单位圆展开正弦波 | 信号 updater 可拼，无现成示例 | 🟡 可拼 |

> 3D 全家桶（ThreeDScene/Surface/ThreeDAxes 共 4 例）超出 2D 目标场景，
> 已在仓库 `docs/05-manim功能差距清单.md` P2 登记，不追。

## 缺口清单（按教学价值排序）

| # | 缺口 | manim 对应 | 工作量 | 建议 |
|---|---|---|---|---|
| ~~1~~ | ~~曲线下面积填充 + 黎曼矩形~~ | `get_area` / `get_riemann_rectangles` | — | ✅ 已由 `riemann_area` 纯现有 API 拼装实现（basic.rs） |
| ~~2~~ | ~~折线/散点图~~ | `Axes.plot_line_graph` | — | ✅ 已由 `line_graph` 纯现有 API 拼装实现（basic.rs） |
| 3 | 网格逐点形变 | `apply_function` + `prepare_for_nonlinear_transform` | M | OpeningManim 级演示效果 |
| ~~4~~ | ~~Angle 角度测量件~~ | `Angle(line1, line2)` | — | ✅ 已由 `angle_demo` 纯现有 API 拼装实现（basic.rs） |
| 5 | 布尔路径运算 | `Union/Intersection/Exclusion` | L | vello 无布尔，需引入 path 布尔库或自研，缓 |
| 6 | 轨迹拖尾件 | PointWithTrace 模式 | S | 低频，可拼 |
| 7 | 画中画缩放 | `ZoomedScene` | L | 低频，暂缓 |

## 入门梯度缺口

manim gallery 的 `ManimCELogo` 仅 ~14 行；ffanim 最短场景 `demo_anim`
约 55 行。R4 规划的 `getting_started.rs`（~15 行：矩形 create + 圆
transform + fade_out）仍待补，是入门体验的最大缺口。

[manim 官方 Example Gallery]: https://docs.manim.community/en/stable/examples.html
