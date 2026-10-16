# 附录：与 manim gallery 对照

以 [manim 官方 Example Gallery]（`docs/source/examples.rst`，5 大节 20 例）
为对照表逐项核对。结论先行：**21 例全部有对应示例**（下表 ✅），
硬缺口全部清零——布尔落地原语、画中画出片层拼装闭环、轨迹/贴轴多边形/
跟随相机三个 🟡 项均已纯现有 API 拼装闭环；3D 类按定位不追。

## 逐项对照

| manim gallery 示例 | 内容 | ffanim 对应 | 状态 |
|---|---|---|---|
| ManimCELogo | 填充形状拼组 + 自定背景色 | `with_fill` + `Theme::background` | ✅ |
| BraceAnnotation | 花括号 + 括注 | `brace_demo`（含方向化花括号） | ✅ |
| VectorArrow | 箭头 + 坐标平面 + 标注 | `arrow_demo` + `Coords::grid_item` | ✅ |
| GradientImageFromArray | 位图导入 + 框选 | `media.rs`（`from_image` + surrounding_rect） | ✅ |
| BooleanOperations | 椭圆并/交/差布尔运算 | `boolean_demo`（`Item::boolean` 四则轮播 + 表达式） | ✅ |
| PointMovingOnShapes | 点沿圆周移动 + 绕点旋转 | `move_demo`（move_along_path）+ `Anim::rotate` | ✅ |
| MovingAround | shift/填充/scale/rotate 动作糖 | `Anim::shift_to/rotate_to/scale_to`（M9.3） | ✅ |
| MovingAngle | 两线夹角弧 + θ 随 tracker | `angle_demo`（弧+双线+θ 标注拼装，角平分线跟随） | ✅ |
| MovingDots | 双 tracker 分别驱动 x/y | `add_tracker` + 信号 updater（M8.3） | ✅ |
| MovingGroupToDestination | 组整体对齐目标点 | `move_to` + group | ✅ |
| MovingFrameBox | 公式框选强调 | `annotate_demo`（circumscribe/surrounding_rect） | ✅ |
| RotationUpdater | dt 自转 updater | 信号节点（求值期纯函数重建，语义等价） | ✅ |
| PointWithTrace | 点运动轨迹拖尾 | `trace_demo`（进度截断参数化重建 + 双 updater 联动） | ✅ |
| SinAndCosFunctionPlot | Axes.plot 双曲线 + 竖线标注 | `sine_demo`（`Coords::plot` + 轴标注） | ✅ |
| ArgMinExample | 函数图像 + 动点跟随 | `Coords::c2p` + tracker（sine_demo 同族） | ✅ |
| GraphAreaPlot | 曲线下面积填充 + 黎曼矩形 | `riemann_area`（c2p 采样拼多边形 + tracker 收敛） | ✅ |
| PolygonOnAxes | 多边形随 tracker 重绘 | `polygon_axes_demo`（双 tracker 重建矩形+轴上拖点 + 面积数字联动） | ✅ |
| HeatDiagramPlot | 折线/散点图 `plot_line_graph` | `line_graph`（polyline + scatter 拼装） | ✅ |
| FollowingGraphCamera | 相机跟随曲线 | `follow_cam_demo`（相机 `move_along_path` 镜像补偿路径，与动点同窗同步） | ✅ |
| OpeningManim | 公式 Write + 网格非线性形变 | 公式族 + `warp_grid_demo`（逐点形变拼装） | ✅ |
| SineCurveUnitCircle | 单位圆展开正弦波 | `circle_sine_demo`（圆拆两半 `plot` + tracker 联动） | ✅ |

> 3D 全家桶（ThreeDScene/Surface/ThreeDAxes 共 5 例）见下方 3D 对照账，
> 已由对照倒推 1.2 API 面立项（docs/07 §1.2）。

## 3D 对照账（官方 gallery 26 例中 5 例真 3D）

对照基线：manim CE v0.19+ 官方 `examples.rst`（2026-10-16 拉取）。
ZoomedScene（MovingZoomedSceneAround）属 2D 画中画，已由 `pip.rs` 清账。

| manim 3D 示例 | 用到的 3D API | ffanim 对应 | 状态 |
|---|---|---|---|
| FixedInFrameMObjectTest | ThreeDScene + `set_camera_orientation(phi,theta)` + ThreeDAxes + `add_fixed_in_frame_mobjects` | `fixed_in_frame`（3D 层 updater 重投影；2D 层数字是不进投影的普通 updater Item，天然固定屏上——零 API） | ✅ |
| ThreeDLightSourcePosition | `Surface(λ(u,v), resolution)` + checkerboard + `light_source` | `three_d_light`（`axes3d` + 球面 `parametric_surface` 棋盘双色派生自主题）；光照非目标（棋盘替代明暗） | ✅ |
| ThreeDCameraRotation | `begin_ambient_camera_rotation(rate)` + `move_camera(phi,theta)` | `three_d_rotation`（theta 挂 tracker 匀速推进 + Smooth 回位，updater 纯查表重投影） | ✅ |
| ThreeDCameraIllusionRotation | `begin_3dillusion_camera_rotation(rate)` | `three_d_illusion`（相位 tracker 匀速推进，updater 求 `theta0 + sin(phase)`，零新动画机制） | ✅ |
| ThreeDSurfacePlot | `Surface(param_gauss, 24×24)` + `set_fill_by_checkerboard` | `surface_plot`（`axes3d` + 高斯面 `parametric_surface` σ=0.7，32×32 棋盘 + opacity 0.8） | ✅ |

由 5 例归并出的 1.2 最小 API 面（4 件，零新动画机制）：
**`Camera3D`**（球坐标纯数据 + view/proj 矩阵）、**`Prim3D` + `project()`**
（每帧纯函数重投影 + 画家算法）、**构造件**（`axes3d` / `parametric_surface` /
`polyline3`）、**fixed_in_frame**（免费）。关键约定：世界系 z-up（manim
同款 phi/theta 语义），光照非目标。详见 docs/07 §1.2「API 面」小节。

## 缺口清单（按教学价值排序）

| # | 缺口 | manim 对应 | 工作量 | 建议 |
|---|---|---|---|---|
| ~~1~~ | ~~曲线下面积填充 + 黎曼矩形~~ | `get_area` / `get_riemann_rectangles` | — | ✅ 已由 `riemann_area` 纯现有 API 拼装实现（basic.rs） |
| ~~2~~ | ~~折线/散点图~~ | `Axes.plot_line_graph` | — | ✅ 已由 `line_graph` 纯现有 API 拼装实现（basic.rs） |
| ~~3~~ | ~~网格逐点形变~~ | `apply_function` + `prepare_for_nonlinear_transform` | — | ✅ 已由 `warp_grid_demo` 纯现有 API 拼装实现（basic.rs） |
| ~~4~~ | ~~Angle 角度测量件~~ | `Angle(line1, line2)` | — | ✅ 已由 `angle_demo` 纯现有 API 拼装实现（basic.rs） |
| ~~5~~ | ~~布尔路径运算~~ | `Union/Intersection/Exclusion` | — | ✅ 已落地 `Item::boolean` 原语（i_overlay）+ `boolean_demo` 对齐场景 |
| ~~6~~ | ~~轨迹拖尾件~~ | PointWithTrace 模式 | — | ✅ 已由 `trace_demo` 纯现有 API 拼装实现（进度截断参数化重建，basic.rs） |
| ~~7~~ | ~~画中画缩放~~ | `ZoomedScene` | — | ✅ 已由 `pip.rs` 纯出片层拼装实现（`render_frame` + 小帧矢量重渲染 + blit，零新增 API） |

## 入门梯度缺口

~~manim gallery 的 `ManimCELogo` 仅 ~14 行；ffanim 最短场景 55 行。~~
`getting_started.rs` 已落地（矩形 create → 变圆 → fade_out，核心三段
各 ~6 行，教程页 first-animation.md 逐段讲解）——示例书同期重构为
「认识 → 上手 → 场景画廊 → 参考」四层（效果画廊按"想要什么效果"组织，
零代码浏览；画廊源码折叠为 `<details>`），入门梯度缺口关闭。

[manim 官方 Example Gallery]: https://docs.manim.community/en/stable/examples.html
