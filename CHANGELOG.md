# 更新日志

本文件记录 ffanim 所有显著变更，格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
版本号自 v0.2.0 起执行 semver 0.x 规则（见 docs/04-项目优化规划.md §3.3）。

## [Unreleased]

### Changed

- chore(docs): §4.3 GIF/WebM 导出实证收官（docs/07）——无需新代码：GIF
  palettegen/paletteuse 两段式（stats_mode=diff + bayer_scale=5）为 M8 既有
  实现，渐变满屏最坏场景实测首帧 MAE 0.62/255（朴素直转 18.02，颜色保真
  29×，体积 2.2× 为最坏上界；帧数/`-loop 0` 循环均实证）；webm(vp9) codec
  M8 已通；webm alpha 随 §1.3 收官
- chore(docs): §4.2 字体文件显式加载实证收官（docs/07）——两侧 API 早已
  落地（评审信息过时）：Typst 侧 `add_font(path)`/`add_font_face(path, index)`
  与描边侧 `Font::from_path` 均收文件路径；开发期宽度对照实证注入字体真被
  选中（等宽 vs 内嵌衬线），补端到端测试 `ffanim-typst/tests/fonts.rs`
  转正（环境探测式：注入可排版/复排缓存一致/face 注入/描边轮廓/越界 face
  报错五断言）；子集化留 P2

### Added

- feat(core): 场景参数声明化（docs/07 §5.2）——serde 轻量方案：typed struct
  即 schema（缺省值 + 类型校验 + 未知字段拦截三职责一处声明），库侧仅
  `load_params`/`params_from_str` 两个加载入口（overlay 语义：JSON 只写要
  覆盖的字段，缺失回退 Default）；Error 增 `ParamsParse`/`ParamsIo`（对齐
  Theme 命名）。场景示范：pendulum.rs 参数化改造（L/g/θ₀/duration 四参数 +
  `--params x.json` 传参 + 物理参数为正自校验），自定义 L=2.5/θ₀=20° 出片
  与解析解 ±1px 吻合。core 单元测试 5 项 + api 冒烟 + 门禁 73 项缺失 0
- feat(core): 命中测试（docs/07 §5.1）——`ffanim-core::hit_test` 纯几何层
  零渲染依赖：`hit_test`（返回命中 id 列表，z 序顶层在前）/`hit_test_first`
  （首个命中）。fill 面走非零环绕（kurbo `Shape::winding`），描边走分段
  最近距离带 `slack` 容差（粗指针/小目标放大命中区）；Group 递归（子件更
  具体在前、组 id 收尾）、clip 门（命中点须落裁剪路径内）、透明排除
  （alpha=0 不命中）、图片矩形命中、退化变换判废。8 项单元测试 + 门面
  re-export + api 冒烟（71 项门禁缺失 0）
- feat(encode): 透明通道导出（docs/07 §1.3）——`EncodeOptions.alpha` 按输出
  格式自动路由带 alpha 编码：mp4 族→ProRes 4444（prores_ks + yuva444p10le，
  剪辑软件标准中间格式，定量码率下 codec/crf/preset 忽略）、webm→VP9 alpha
  （yuva420p + auto-alt-ref 0）、gif 报可读错误；配套门面 `render_to_mp4_with`
  （细粒度编码选项）+ re-export `EncodeOptions`/`Quality`/`RenderBackend`。
  渲染端零改动：bg alpha=0 数据驱动产出透明底（GPU/CPU 双后端 straight
  约定像素级实证统一）。正式测试 3 项：mov 硬断言（ffprobe 实证 + 像素三区
  透明底/半透明 ±1 量化/不透明精确）、webm 探测式（codec=vp9 硬断言，alpha
  在部分 ffmpeg 环境静默降级为环境报告）、gif 报错。顺手修复两个真 bug：
  段编码失败静默（spawn 的 Result 未收集，拖到 concat 才报 Invalid data）与
  alpha 段容器误用 mp4（ProRes 走 mov）

- feat(examples): FixedInFrameMObjectTest 拼装闭环——3D 对照账 5/5 全部收官——
  `fixed_in_frame.rs`（零新 API）：3D 层单进度 tracker 派生同窗动画（轴绕 z
  转 90° + 圆平移到 (1,1,0)，updater 每帧重投影），2D 层数字
  （`decimal_signal` 挂进度 tracker 滚动 0°→90°）是不进投影的普通 updater
  Item，天然固定屏上无需固定层机制；105 帧出片抽帧目检通过（数字位置全程
  固定、数值与 3D 旋转联动、轴对角翻转与圆落点正确）
- feat(core): 3D 构造件 `axes3d` + `parametric_surface`（docs/07 §1.2）——
  `Axes3DOpts`（三轴 + z=0 方格网格，颜色/线宽显式传入零硬编码）+
  `SurfaceStyle`（棋盘双色/单色 + opacity + 描边，顶点采样缓存再拼面片）；
  4 项单元测试 + api 冒烟扩至 15 图元。同步出片两例：`three_d_light.rs`
  （ThreeDLightSourcePosition 对照——球面棋盘，光照非目标由棋盘替代明暗）
  与 `surface_plot.rs`（ThreeDSurfacePlot 对照——σ=0.7 高斯面 32×32 +
  opacity 0.8），3D 对照账 4/5 ✅
- feat(examples): ThreeDCameraIllusionRotation 拼装闭环（docs/07 §1.2 3D 对照第二例清账）——
  `three_d_illusion.rs`：相位 tracker 匀速推进（rate=2 rad/s × 4s Linear），
  updater 纯函数求值 `theta = theta0 + sin(phase)`（幅度固定 1 rad，manim
  `begin_3dillusion_camera_rotation` 同款语义），零新动画机制；150 帧出片抽帧
  目检通过（起始 −45°/右摆峰 +12°/左摆谷 −102°/回摆中，峰谷差 114° 合理论）
- feat(examples): ThreeDCameraRotation 验收示例（docs/07 §1.2 对照首例清账）——
  `three_d_rotation.rs`：三轴 + z=0 方格网格 + 单位圆 `polyline3` 拼图元表，
  相机方位角挂 tracker，ambient rotation（0.1 圈/s×3s Linear）+ move_camera
  （Smooth 回位）+ 尾停，updater 每帧 `project` 纯查表重投影；150 帧出片
  抽帧目检通过（起步机位/绕 z 自转 72°/回位一致），相机动画零新机制
- feat(core): 3D 投影模拟核心（docs/07 §1.2）——`ffanim::space3` 新模块：
  ① `Camera3D` 球坐标相机（manim 同款语义 phi=极角/theta=方位角，默认机位
  75°/30°/r=10/fov=45°），`eye()`/`view_matrix()`/`proj_matrix(aspect)` 纯
  函数；phi→0/π 视线与 z 轴共线时 up 自动换轴防退化；② `Prim3D` 图元表
  （Polyline/Polygon）+ `PrimStyle`（line/face 便捷构造）+ `polyline3`/
  `polygon3`；③ `project(&prims, &cam, w, h) -> Vec<Item>` 每帧纯函数重投影：
  view*proj → `mul_vec4` 齐次变换 → 手动除 w（顺带整条丢弃相机身后图元，
  manim 同款简化不做视锥面剪裁）→ NDC→像素（y 翻转）→ 画家算法（图元质心
  NDC z 排序，远的先画，相交面不剖分）——产物即普通 2D Item，描边生长/
  fade/clip 全部免费复用；相机动画 = 字段挂 tracker + `Anim::set_value`，
  零新动画机制。6 项解析解测试（眼位球坐标/光轴点落屏幕中心/z-up 映射屏幕
  上方/画家算法远先画/身后图元丢弃/俯视退化矩阵有限）+ 门面冒烟（产物注册
  时间轴淡入）。坑：齐次乘法是 `mul_vec4` 非 transform_point4（glam 0.34）
- feat(core): 3D 线代件（docs/07 §2.2，glam 修正案）——引入 glam 0.34（f64
  全套：零传递依赖、仅启用 `std`+`f64` 精度特性、锁小版本对冲 API 变动），
  `ffanim::math` re-export `DVec3`/`DMat4`/`DQuat` 为稳定数学面；相机/投影
  构造在此**锁死约定**（右手系 + OpenGL 深度 [-1,1]，manim 对齐）：
  `look_at`/`perspective`/`orthographic` 稳定别名，上层 1.2 投影管线不再
  面对 rh/lh × NDC 约定选择题；6 项约定锁定测试（叉积右手性/旋转/look_at
  视空间/透视与正交深度边界/四元数一致性），glam 小版本约定漂移即 CI 红。
  沉淀约定坑：透视链路必须 `project_point3`（含 w 除法）——
  `transform_point3` 为仿射专用（断言 w 行且丢弃 w），返回 clip 空间 z。
  修正原评审「手写不引 glam」定案：glam 零传递依赖事实下不成立，手写
  真正成本在投影矩阵约定正确性而非代码量
- feat(render): clip 裁剪原语（docs/07 §1.1，透镜/区间显示/滚动窗口基元）——
  `Item` 新增 `clip: Option<BezPath>` 字段与 `with_clip` 构建器（路径为
  Item 局部坐标，随 `transform` 一起变换；Group 作用于整棵子树）；场景
  遍历改事件流 `walk_events`（`Draw/PushClip/PopClip` 严格栈式配对），
  CPU（vello_cpu `push_clip_layer`，clip 变换 push 时刻固化）与 GPU
  （vello `push_clip_layer` 显式传 transform，层栈不保存变换）双后端
  映射一致；形变产物继承 from 侧裁剪域。验收：CPU 像素级测试（叶子裁剪
  + Group 随变换子树裁剪）、GPU 冒烟、`examples/lens.rs` 放大镜透镜
  （整帧场景克隆 × 镜心不动点放大仿射 + 圆 clip 一次渲染合成，出片
  282 帧抽帧目检——镜内矢量级放大含线宽 2.5×，镜外原样，透镜错位
  视觉正确）
- feat(core): ODE 数值积分工具集 `ffanim_core::math`（docs/07 §2.1）——
  `rk4(f, y0, t0, t1, steps)` 四阶 Runge-Kutta 构建期预积分 + `Trajectory`
  均匀网格查表（`at(t)` 线性插值、越界钳位，动画窗口长于积分窗口时
  定格端点）+ `component(k)` 分量序列（喂 `Axes::plot` 相图）+ `bisect`
  二分求根；完全匹配两阶段时间轴模型（构建期积分、渲染期纯查表）。
  验收：抛体运动对解析解、无阻尼单摆能量守恒（8s 摆动漂移 < 1e-6）、
  `examples/pendulum.rs` 出片（支点+杆+摆锤+摆角弧 updater 查表重建）
- feat(render): PNG 序列帧导出（docs/07 §4.1）——`write_png_sequence`
  （`{dir}/{n:04}.png`，原子计数器动态分派帧号防首帧排版冷启动负载
  倾斜，worker 数为核数一半不超订阅）+ 门面 `render_to_png_frames`
  （全片）/`render_to_png_frames_range`（片段，传全片帧号偏移与 mp4
  对帧）；渲染走与出片相同的 `FrameBackend::auto` GPU 优先策略，PNG
  编码纯 Rust 不依赖 ffmpeg——下游剪辑/审片标准中间格式与论文插图
  逐帧提取；端到端测试覆盖文件名对帧与像素断言
- feat(bench): 性能回归门禁（docs/07 §6.1）——新增编排/求值维度基准
  `crates/ffanim-core/benches/pipeline.rs`（`compile_60_items` 10.5µs /
  `eval_frame_1080p` 3.5µs，与渲染维度 render.rs 同负载互补成完整
  管线）；`just bench-gate`（`scripts/bench_gate.sh`）全维度对阈值
  断言（>50% 硬回退即红，阈值 = docs/benchmarks.md 基线 × 宽松系数
  防环境波动），数字入库 docs/benchmarks.md
- feat(render): GPU 默认出片 + 自动降级——新增后端选择器
  `ffanim_render::FrameBackend`（枚举 `Gpu/Cpu` 双分支，均 `Send`，出片
  管线经 `Mutex` 单实例共享，GPU 提交串行但帧求值仍分段并行）：
  `auto()` 启动时探测 wgpu 适配器，成功走 Vello GPU、失败打 warn 自动
  降级 vello_cpu（调用方零分支，无 GPU 是常态而非错误）；门面新增
  `render_to_mp4_cpu` 强制 CPU 出片（逐字节确定，供回归比对/CI/无 GPU
  环境显式锁定）；criterion 新增 `gpu_render_1080p` 基准（本机实测
  1080p 单帧 CPU 35.7ms → GPU 2.73ms，≈13×，全管线实时率门禁余量
  大幅增厚）；后端自动选择端到端测试覆盖
- feat(api): Phase 2 Stage 糖层落地（docs/07 评审路线，fourier.rs 验收）——
  ① `ffanim::stage::Stage` 场景工作台：`from_theme_path/with_theme` 一次
  装配（主题加载 + Timeline + TypstEngine + CJK 字体注入），`text_at/
  formula_at` 以像素意图（屏上字号 + `Anchor` 锚点）一步完成「排版→拆件→
  落位→注册」，`write/fade_in` 一行排入组动画，`tl()/into_timeline()`
  原语逃生舱随时绕过（纯组合器零新渲染能力）；② D3 落位意图原语：
  `Formula::to_items_at(cx, cy, font_px, Anchor)`（`Formula` 新增
  `font_size_pt` 记录排版字号），替代四拍写法；验收：fourier.rs 236→
  208 行（样板区 −50%，tracker/相机/诊断原语区不压），出片 16.67s 与
  原版等价（strip/onion 目检），walkthrough 同步 Stage 叙述并新增
  「第 0 幕 · 一次装配」
- feat(api): Phase 1 API 地基落地（docs/07 评审路线）——① 组动画一行
  排入 `Timeline::animate_group(&ids, Anim::create, step)`：对已注册句柄
  逐个套用同一动画模板（`Anim::create`/`fade_in` 等构造器可直接作模板），
  省去「收集 ids → map 构造 Anim → group」三步样板；② 自定义速率函数
  开放：`Timeline::register_rate(f) -> RateFunc`（新增 `RateFunc::Custom(u16)`
  注册表索引变体，保持 `Copy`/序列化语义），注册表随 compile 整体搬入
  `CompiledTimeline.rates`（Arc 共享，渲染期分段并行查表），`RateFunc`
  封闭枚举的缓动天花板拆除；③ 新公开导出 `RateTable`，端到端集成测试
  覆盖（注册二次缓入 → 中点帧位移精确断言）

### Changed

- fix(encode): 出片编码并行度收敛（修复笔记本爆载过热重启）——分段 ffmpeg
  此前无线程上限，libx264 每进程默认 ≈1.5×核数 帧线程，叠加
  `available_parallelism` 全核分段（16 段 × 24 线程 ≈ 400 逻辑线程）+
  16 渲染 worker，笔记本 APU（CPU 与 iGPU 同散热槽）长时间满载触发过热
  保护重启；现每段 ffmpeg 强制 `-threads 1`（并行已由分段承担，段内帧
  多线程纯浪费），默认段数收敛为核数一半（段数 × 2 恰好铺满逻辑核不超
  订阅），`EncodeOptions::segments` 仍可显式全速；实测 fourier GPU 出片
  平均负载 ≈4.7 核（user/real），出片 9.2s 正常
- feat(render)!: ffanim-render `gpu` feature 转为默认开启——出片默认后端
  从 CPU 改为 GPU 优先（`render_to_mp4/render_to_mp4_frames` 经
  `FrameBackend::auto()` 自动选择，无 GPU/驱动缺失自动降级 CPU）；无
  GPU 环境编译可 `--no-default-features`（此时 `FrameBackend` 仅剩
  `Cpu` 分支，出片一律 CPU）
- feat(api)!: **Theme 解析前置（breaking）**——颜色在 `Theme::from_path`
  /`FromStr` 加载点一次性解析，取色 API `background()/foreground()/accent()`
  从 `Result<Color>` 收窄为 `Color`；非法十六进制在加载时报错（错误消息
  带字段名定位），不再泄漏到调用点——示例/CLI/测试 ~100 处 `?`/`unwrap`
  摩擦全部消失；docs/07《核心能力缺口与 API 优化评审》入库（能力缺口
  7 维度清单 + API 冗余根因与 Stage 糖层方案 + 4 Phase 落地路线，
  3D 路线定为投影模拟、真 3D 光栅化列为非目标）

### Added (book)

- feat(book): 示例书重规划批 3——实战与参考落地：新增**实战拆解**
  页（fourier.rs 按五幕创作流程拆解：脚本先行 → 标题/公式/坐标系/
  tracker 逼近/出片诊断，源码加 5 个幕级 ANCHOR 子锚点逐段引用）与
  **配方 FAQ** 页（六个最小原语拼装配方：轨迹拖尾 / 画中画 / 布尔
  挖洞 / 网格形变 / 相机跟随补偿 / 数字联动，节选代码 + 完整场景
  链接）与**词汇速查**页（门面 56 项公开 API 按 9 类分组，交叉链接
  课程与画廊）；SUMMARY 骨架加「实战与参考」Part
- feat(book): 示例书重规划批 2——分课教程六课落地：新增 `lessons.rs`
  六课素材（每课一函数 ≈30 行，`ANCHOR: <课名>` 标注，gally Asset 模式
  只出片拷媒体）——第 1 课形状与样式 / 第 2 课动画词汇 / 第 3 课文字与
  公式 / 第 4 课坐标系与函数图像 / 第 5 课布局与强调 / 第 6 课镜头语言；
  SUMMARY 新增「分课教程」Part，六课手写页 lesson-*.md 统一「视频 →
  全函数代码 → 要点 → 试着改一改 → 下一课」结构；first-animation 与
  index 接入分课教程入口
- feat(book): 示例书重规划批 1——从「源码堆砌的画廊」重构为四层
  （认识 → 动手写动画 → 场景画廊 → 参考）：新增**效果画廊**手写页
  （按「想要什么效果→词汇」组织 11 类，零代码浏览 + 代表视频）与
  **第一个动画**教程页（环境准备 + 逐段讲解 + 改练清单）；新增
  `getting_started.rs` 入门素材（矩形 create → 变圆 → fade_out，核心
  三段各 ~6 行，对标 manim SquareToCircle，关闭 manim 对照「入门梯度
  缺口」）；画廊章节全量源码折叠 `<details>`；ffanim-gally 改混合模式
  （`ChapterKind`：画廊章节生成页面 / 素材文件只出片供手写页引用，
  SUMMARY 按 mdBook Part 骨架生成，手写页永不触碰）；pip.rs 登记为
  「创作示例：画中画」章节
- feat(gally): 示例书工具链——`ffanim-gally` 转型为 mdBook 书源生成器
  （rust-lang 官方 mdBook 0.5.4）：扫描 `crates/ffanim/examples/*.rs` 自动
  提取标题/场景清单/doc 摘要（零硬编码数据源），渲染缺失产物并拷贝
  mp4 → 生成 `docs/book/src/SUMMARY.md` 与各章节页（`## 场景` + doc 摘要 +
  `<video>` + `{{#include}}` ANCHOR 锚点代码段，examples 源文件加
  `// ANCHOR: 场景名` 标注）；`just book` 一键构建 / `just book-serve`
  热重载预览；手写 `book.toml`/`index.md` 入库，生成物 gitignore
- docs: 附录「与 manim gallery 对照」——官方 gallery 5 节 20 例逐项核对：
  10 例已覆盖 / 5 例可拼 / 5 例真实缺口（面积填充+黎曼矩形、折线图、
  网格逐点形变、Angle 角度件、布尔运算），结论同步 docs/06 R4

### Changed

- refactor(examples): 18 个内置场景迁移至门面 crate `examples/`（对齐
  manim `example_scenes` 形态）——`basic.rs`(8)/`typography.rs`(5)/
  `assemblies.rs`(4)/`tables.rs`(1) 四文件分组，单文件多场景 + argv 寻址
  （缺省全渲染，≙ `manim -a`），只依赖门面公开 API；18 场景 timeline
  JSON 与迁移前 md5 逐一比对 18/18 一致（零语义漂移）
- refactor(cli): `ffanim-cli` 退回纯薄壳——删除场景绑定子命令 12→4
  （仅 new/audio/info/diff），删 `scenes.rs`（1283 行）与孤儿依赖
  （render/encode/typst/preview）；门面补公开导出 `ItemId`/
  `DecimalNumberOptions`；`just demo`/`demo-formula` 改走 examples 形态

## [v0.4.0] - 2026-10-14

M9 manim 功能对齐收官：P1 差距清单 14 项全部落地（M9.1~M9.18），
渲染/编码/工具链同步强化。

### Added

- feat(core): M9.18 表格（manim `Table` 对应）——`ffanim::table(headers, rows_data, &Font,
  &TableOptions)` core 侧 `Font::outline_text` 逐格排版拼组（列宽/行高自动或指定、
  格内居中、网格线内外开关 `include_outer_lines`；弃 Typst 整块 `#table()` 保逐格
  动画句柄）→ `TableSpec`：整表 `table` / 行组 `rows`（lagged_start 错峰出场）/
  `cell_rects` + `row_rect`/`col_rect` 并集（高亮底板与 `surrounding_rect` 圈注定位）；
  z 序 = 注册序（高亮底板先于行组注册垫底）；table_demo 场景验收（行组错峰淡入 +
  第二行高亮 + 示例格圈注，末帧像素探针三要素实证）
- feat(encode): M9.17 GIF/WebM 输出 + 画质预设（manim `-ql/-qm/-qh/-qk` 对应）——
  `Quality` 预设表（ql=854×480@15 / qm=720p30 / qh=1080p60 / qk=2160p60：
  定输出分辨率 + fps（时间轴按预设 fps 重建）+ 分格式 CRF）+ 输出扩展名路由
  `.mp4/.gif/.webm`：GIF = 并行编中转 mp4 → palettegen/paletteuse 两遍调色
  （diff 调色 + bayer 抖动）+ `-loop 0` 无限循环；WebM = libvpx-vp9 分段直编 +
  concat 流复制（`-row-mt`/`-cpu-used 4`）；CLI `--quality` + GIF×音轨互斥校验；
  验收 ffprobe 实证三格式分辨率/fps/帧数 + GIF NETSCAPE2.0 循环扩展
- feat(typst): M9.16 按词着色 t2c（manim `Text(t2c=…)` 对应）——`build_t2c_markup`
  纯函数（全出现/对序优先/非重叠匹配，`#text(fill: rgb()) [词]` 局部色覆盖全局
  `#set text(fill:)`，markup 特殊字符保守转义视觉原样）+ `TypstEngine::text_t2c`
  （经 `text()` 走 LRU 缓存，同输入 0 开销）；词未在原文出现报错（重叠让位合法）；
  t2c_demo 场景验收（中文重复词全着色 + 英文特殊字符句，第二高亮色
  `Color::lerp(前景, 强调)` 派生）
- feat(typst): M9.15 数字滚动 DecimalNumber（manim `DecimalNumber` + ValueTracker 对应）——
  `DecimalNumberOptions`（小数位/正负号/千分位/字号/颜色）+ `TypstEngine::decimal`
  （单发排版，bbox 中心精确锚定）+ `decimal_signal`（自含 `Mutex<TypstEngine>` 的
  updater 信号工厂，`Fn(&[f64]) -> Item + Send + Sync`，逐帧 markup 重排：math 模式
  探针实证降级——逗号/正号带运算空隙 +14/+24px、数字高度随位数波动）；decimal_demo
  场景验收（千分位滚动 + 带符号滚动，tracker + `Anim::set_value` 驱动，出场 fade_in）
- feat(core): SVG 导入——`from_svg_str`/`from_svg_file`（usvg 0.45 → kurbo 路径 → `Shape::Group`，渐变/图案/文本 warn 跳过）；`Item::from_image` + `ImageAsset`（预乘 RGBA8，`Arc` 零拷贝共享）；`Paint.opacity` 整体透明度系数（组合递归传导子项，淡入/淡出对组合与图片生效）
- feat(render): CPU/GPU 双后端图片渲染（vello_cpu `Pixmap::from_parts` + 透明度层；vello `draw_image` + `push_layer`）；组合淡入时按子项包围盒并集取中心（替代原点，消除组合动画变形）
- feat(examples): `media.rs` 验收示例——SVG 组淡入 + 位图淡入后放大 3×（90 帧，前景像素实证）

### Fixed

- fix(typst): 中文字体选择错误——`find_cjk_font` 改返回 `(路径, face 序号)` 且
  `fc-list` 排序修正：原仅按文件名排 bold/italic，在本机会选中
  `NotoSansCJK-Black.ttc`（文件名不含 bold）且其 face 0 为**日文 JP Black**，
  复/沿等字用日文写法 + 超粗重量渲染（table_demo/delimiter_demo 目检实证）。
  现排序键 = 常规体（族名/样式无重量后缀）→ SC 简体族 → 非等宽变体（选中
  Sarasa UI SC Regular）→ 路径字典序；新增 `TypstEngine::add_font_face`（单
  face 精确注入，CJK fallback 结果确定，避免 ttc 全量注入时选错 face）；全部
  调用点迁移 `Font::from_index` / `add_font_face`

### 新增（M8：功能补齐）

- feat(core): M8.1 相机系统——`Camera`（center/zoom/rotation）+ 相机即 Item
  （不可见对象，transform 即 world→screen 仿射，docs/02 §3.4 兑现）+
  `Anim::transform_to` 通用"变换到目标"词汇（段前取基线/段中插值/段后驻停，
  from 由构建期推演）；fourier 示例加推近镜头验收段（zoom 1.4 推镜，
  非背景像素 5.01%→6.50% 实证放大）+ 相机求值测试 3 项
- fix(render): bench 文件豁免 missing_docs（criterion 宏展开函数无文档）
- feat(core): M8.3 组动画——`Timeline::group`（固定步长错开）+ `lagged_start`
  （间隔 = ratio × 前段时长，manim lagged_start 语义），与 play 共享构建期推演；
  fourier / formula_demo 的 Write 重写为 group，调度逐帧等价（快照 PASS），
  消除 `overlap(dur-step)` 手工对齐（M3 踩坑根源）
- feat(typst): M8.2 富文本 Text——`TypstEngine::text`（markup 模式，`*粗体*` 混排）
  + `TextOptions`（字号/行距/限宽换行），产物复用 Formula 管线（to_items/fit_transform
  原样可用）；`TypstEngine::add_font`（ttc 全 face 注入，缓存键含字体代号防串）+
  `find_cjk_font` 系统中文字体查找（env FFANIM_CJK_FONT → fc-list :lang=zh →
  路径扫描），fourier 的 CJK_FONT 硬编码与 text_item 助手删除；新增 text_demo
  场景（中英混排限宽不出框：末帧内容 bbox 距画布边缘 ≥165px，0 触碰实证）+
  测试 5 项；MathWorld 字体集动态化（基础 typst-assets + 注入）

- feat(api): M8.6 API 覆盖率门禁 + 集成测试补全 + ffmpeg stderr 吞错修复——scripts/api_coverage.sh（40 个门面公开项全覆盖，缺失退出 1，已挂 just ci）；crates/ffanim/tests/ 3 文件 26 用例（timeline 9 调度语义 / render 6 像素探针 / api 11 门面冒烟，ffmpeg 缺失时打点跳过）；ffanim-audio decode stderr null→piped（log::debug 全量记录，失败尾行并入 Error::Encode）

### M9.1 公式 diff 引擎升级（2026-10-12）

- `match_formulas` 用 similar（Myers）重写替代手写 LCS，新增 move 二部配对：同文本的删除/插入对识别为移动（`a+b=c → b+a=c` 交换子式做 Transform 而非消失重现，乱序下标配对）
- 形状片段（无源文本）用侧别唯一占位符，恒不跨配对
- 新增 `ffanim diff <a> <b>` CLI 子命令：预检 TransformMatchingTex 调度计划（配对/淡出/淡入）
- 3 单测（交换 move / 原位替换 / 形状恒不配对），门禁全绿

### M9.2 片段导出（2026-10-12）

- 门面新增 `render_to_mp4_frames(tl, out, frames)`：帧区间半开裁剪，帧号平移复用同一条 ffmpeg 管道（encode 零改动）；越界区间报 `Error::Other`
- CLI `render --section "起..止秒"`：秒→帧四舍五入 + clamp 到全片；与 `--audio` 同用时提示音轨为完整音轨
- 验收：demo_anim 5.8s 全片导出 1..3s 片段，ffprobe 实测 2.0s；3 解析单测 + 1 集成测试
- 顺手修复 ci-gpu 配方存量编译损坏（peniko `Blob::new` 需 `Arc<dyn AsRef<[u8]>>`，改拷贝进 `Arc<Vec<u8>>`）

### M9.13 定界符统一：bracket / parenthesis / 双线 bracket 迁 Typst MATH 装配（2026-10-14）

- feat(typst): `brace.rs` → `delimiter.rs`——`Delimiter` 枚举（Brace/Bracket/Paren/DoubleBracket）
  + 共享装配核心（深度探针/归一化仿射/精调缩放/LRU 缓存），`brace`/`brace_with_text` 保留别名
- 新实证：bracket `\[`、paren `\(`、双线 `⟦⟧` 配对语法进 `lr(size:)` 触发 NewCMMath
  GlyphAssembly 装配；装配档位离散（bracket 最深预组档恰 3em，跨度差 ~0.6%）→
  归一化后按实测 bbox 非均匀精调精确落到 [0,span]×[0,depth]
- 验收：delimiter_demo 场景（四类定界符贴横条四侧 + 中文括注）+ ffanim-typst 单测 9 +
  ffanim 集成 1（workspace 172 测全绿）

### M9.14 箭头装配：Arrow 迁 Typst MATH stretch 装配（2026-10-14）

- feat(typst): 新增 `arrow.rs`——`ArrowKind`（Single/Double）+ `TypstEngine::arrow(from, to, kind)`
  两点式（manim `Arrow(start, end)` 对齐）；`stretch(->, size: #Npt)` 触发 NewCMMath
  GlyphAssembly 拉伸，跨度精确吸收（宽零偏差——无 delimiter 档位离散，免精调仿射）
- 新实证：头高/杆厚 ∝ 字号（0.520 / 0.048，pt 空间精确线性，探针定标）；单头 10 片 /
  双头 19 片（2 头 + 17 杆）；关系符进 `lr(size:)` 不拉伸（w1=w2 实证）→ 必须走 `stretch()`
- 几何：头高 = clamp(span×0.15, 10, 24) px（manim capped 语义同族），字号反推；
  归一化（原点=from、杆沿 +x、中线 y=0、头在 +x 端）+ 显式 6 元矩阵弦角复合
  （c=dx/span, s=dy/span），头尖精确闭合 to；跨度 <20px 报错；`with_fill` 着色 +
  `fade_in` 出场（填充字形禁 create）
- 验收：arrow_demo 场景（水平单头/斜向单头/水平双头依次淡入 + 中文括注）+
  ffanim-typst 单测 6 + ffanim 集成 1（workspace 184 测全绿）

### M9.12 manim 对齐第九批：P1-8 Brace 花括号 + 括注文本（2026-10-13，2026-10-14 改 Typst 装配机制）

- feat(typst): `TypstEngine::brace(&target, dir, buff)`——沿目标 bbox 指定侧
  （Down/Up/Left/Right）生成花括号，开口朝对象、中尖角指外；规范形（跨度沿 +x、
  开口 -y）+ 四方向旋转定位
- 形状机制 = **Typst OpenType MATH 装配**（NewCMMath 字体 GlyphAssembly：顶钩 +
  可重复延长臂 + 中尖角 + 底钩，`lr(size: #Npt, \{)` 排拉伸花括号，片间按 MATH 表
  MinConnectorOverlap 重叠防接缝）：跨度由 `lr(size:)` **精确到像素**（装配延长臂
  吸收），深度 ∝ 字号严格线性（深度/字号比运行时探针实测，零硬编码魔数；字号随
  深度定标 → 钩/尖角比例保真）；**闭合填充字形**（真实粗细过渡），装配片合并为
  单路径（同色不透明填充重叠区无瑕疵），归一化仿射（pt→px + 旋转 -90° + 平移）；
  产物坐标语义与首版一致（x∈[0,span]、y∈[0,depth]、开口线 y=0）
- 演进史：v1 手绘描边中心线（无粗细过渡，弃）→ v2 manim CM 模板移植（manim 0.9
  brace.py 路径模板 + 线性段拉伸，字形质量达标但深度为近似公式）→ v3 Typst 装配
  （跨度/深度完全解耦、拼接质量由字体表保证、删 ~50 行几何代码与模板字符串）；
  三版均为 CM 系字形（manim/LaTeX/Typst 同源）
- **填充字形不适用 create 描边生长**（可见逐段描轮廓过程），出场用 fade_in
  （manim 对 Brace 亦用 GrowFromCenter 而非 Create）
- `TypstEngine::brace_with_text(...)` 返回 (brace, 文本) 散件对——文本置于尖角
  外沿居中（间距 0.45×字号，manim `brace.get_text` 对应）
- brace / brace_with_text 两条 log::info 埋点（目标 bbox/跨度/深度/结果 bbox、标签/bbox）
- 语法要点（探针实证）：`{` 在 math 里是分组符须写 `\{`；`lr(size:)` 必须加 `#`
  且用绝对长度（em/pt；百分比被忽略）；Typst **无 `px` 单位**（px→pt 换算 ×0.75）
- 验收：单测 4 项（下方几何 0.5px 容差/四方向外沿/括注居中/缓存命中零成本）+
  集成 1 项（几何断言 + together 编排三相处数）；brace_demo 场景 5.1s 出片
- **调研与获取**：GitHub raw/jsdelivr 均被墙，改走清华 pypi 镜像下载 manim wheel
  解包提取官方路径模板（模板常量与源文件逐字节比对一致）；像素级剖面实证新字形
  端钩卷曲、直臂、中尖下探至 bbox 下沿（y=362 vs 363.6 抗锯齿差）

### M9.11 manim 对齐第八批：P1-5 TransformMatchingShapes 任意片段结构匹配（2026-10-12）

- feat(core): `ffanim::match_shapes(&[Item], &[Item]) -> MatchPlan`（match_shape 模块）——
  文本缺失路径片段（SVG 字形/几何图形）的几何签名配对，manim `TransformMatchingShapes` 对应
- **几何签名**：base path 平移归零 + 0.01pt 精度序列化——同形异位必配对（outline_text/SVG
  的烘焙位移、arrange 的 transform 挂位均可），异形不配对；样式不参与，Group/Image 不参与
- 算法 = 按签名 FIFO 二部配对（重复件按出现序对应，输出确定性排序，与文本 diff 不同天然
  允许乱序移动）；粒度调用方决定（逐字形 / 逐图形）
- **MatchPlan 上移统一**：struct 从 ffanim-typst::matching 移至 ffanim-core::match_shape，
  typst 侧 re-export 保持 `ffanim_typst::MatchPlan` 公开路径零变化（文本配对与几何配对同型）
- match_shapes 带 log::info 埋点：输入片段数 → 配对数（含异位移动）/淡出/淡入
- 验收：shapes_demo 场景（"MATH" → "MAST" 逐字形：M/A 原地形变、T 异位滑动、H 淡出、
  S 淡入）4.3s 出片目检；单测 7 项 + 集成 1 项（三件套交换配对 + 窗口三相处数 + 中点位置）

### M9.10 manim 对齐第七批：P1-4 MoveAlongPath 沿路径移动（2026-10-12）

- feat(core): `Anim::move_along_path(id, &BezPath)`（AnimKind::MoveAlongPath）——弧长匀速
  参数化采样（kurbo ParamCurveArclen + inv_arclen，曲线段精确反解），缓动作用于进度
- **语义**：位移 = 弧长采样点 - 路径起点（起点对齐，与对象初始位置无关，manim
  MoveAlongPath 同义）；段后驻停路径终点；prepare 基线快进到路径终点偏移，后续动画以
  终点接续；路径保留在编译表内逐帧求值（无法烘焙为 TransformTo 直线插值）
- **存量 bug 根修（transform_at）**：段前取 from 的分支此前无条件执行，顺序串接两段位姿
  动画时后段 from 覆盖前段全程插值（实证：两段 shift_to，frame0 x 应为 0 实为 90）——
  加 touched 标记，段前回退仅对该对象首个生效动画生效
- path_util 新增 `sample_path`（弧长比例采样）/`path_length`；带 log::info 埋点两条
  （构建期"解析"：轨迹长/bbox/起终点/基线推进/dur；"时序"：生效窗口）
- 验收：move_demo 场景（S 形贝塞尔轨迹 create + 小球贴线移动）5.2s 出片目检；
  path_util 单测 4 项 + eval 三相/接续单测 3 项 + 集成 1 项（折线弧长逐帧断言）

### M9.9 manim 对齐第六批：P1-13 代码块高亮 code（2026-10-12）

- feat(typst): `TypstEngine::code(body, lang, &CodeOptions)`——Typst raw 块 + syntect 语法高亮，
  产物复用 Formula 管线（to_items/fit_transform/逐字形动画原样可用）；LRU 缓存键加 lang/theme
  代号防同文互串；`lang` 白名单清洗 + 正文含反引号串时围栏自动升级（至少 3 级）防破块
- 内嵌深/浅两套 tmTheme（`CodeTheme::Dark/Light`，GitHub Dark/Light 色板，经 `World::file`
  虚拟文件加载，token 色属排版引擎资产不走场景主题 JSON）
- **关键语义（typst-library raw.rs styled()）：raw 高亮只给命中 scope 的 token 设显式 fill，
  普通文本继承外层 `#set text(fill:)`（默认黑，深色画布不可见）——compile_code 显式填入
  `CodeTheme::foreground_hex()` 作正文前景色**
- code 带 log::info 埋点：语言/行数/字号/主题 → 宽高/片段数/着色种数/耗时
- 验收：code_demo 场景（Rust fib 逐字形写出 + surrounding_rect 圈选 + Python 块淡入）
  4.8s 出片目检（关键字/函数/类型/数字/字符串多色 + 前景色白正确）；单测 9 项 + 集成 1 项

### M9.8 manim 对齐第五批：P1-7 标注复合件 surrounding_rect / underline / cross / background_rect（2026-10-12）

- feat(core): 四个标注复合件几何构造器（P0-1 基元 + P0-2 布局落地后的薄糖，docs/05 P1-7）：
  `Item::surrounding_rect(&target, buff)`（bbox 四边外扩 buff 的圈选框）/
  `Item::underline(&target, buff)`（视觉底部下方 buff 处等宽横线）/
  `Item::cross(&target, buff)`（外扩框对角双线，单路径双子路径，create 逐条描出）/
  `Item::background_rect(&target, buff)`（背景板，注册须先于目标 = 画在下）
- 新增 `Item::group(items)`（manim VGroup 构造侧）：公式等排版散件拼组后整体标注/布局；
  拼组仅作几何用途时不注册（Shape::Group 此前只能手拼 struct 字面量）
- 语义要点：标注基于**注册基线** bbox（含 transform）；目标有位姿动画时标注挂相同
  shift_to 同步（annotate_demo 第 1 段演示）；半透明背景板用 `Color { a, ..accent }`
- 验收：annotate_demo 场景（公式圈选+同步平移 / 下划线 / 叉划错误式子 / 背景板，
  7.2s/216 帧/41 动画）出片目检 4 帧通过；埋点核对（目标 bbox → buff → 结果几何）；
  单测 +6 项 + 集成 `annotate_标注复合件编排`（含"compile 后基线推进到 to 态、
  框仍罩目标 ±buff"断言）；SKILL.md 词汇表同步

### M9.7 manim 对齐第四批：P1 薄糖批 Uncreate / Together / Succession / 方向家族 / add_group / save_state·restore（2026-10-12）

- feat(core): docs/05 差距清单 P1-1/2/3/10/11 五项同批落地（全部构建期薄糖，求值路径仅新增
  Uncreate 分支与 FadeIn 参数化，零既有语义改动）
- `Anim::uncreate(id)`：create 镜像（描边按进度收回 + 填充淡出，段后隐藏）——`AnimKind::Uncreate`
  变体，求值复用 `apply_create(1-alpha)`
- FadeIn 方向家族：`FadeIn` 变体参数化 `{ shift, scale_from, spin }`（默认 0.9/0 保持现行为），
  新构造器 `fade_in_from(id, dir, dist)` / `grow_from_center(id)` / `grow_from_edge(id, dir, dist)` /
  `spin_in(id, 弧度)`，统一走 `enter_transform`（位移余量 + 绕 bbox 中心缩放/旋转收正）
- 组编排糖：`tl.together([...])`（同窗对齐，游标取最长 dur——规避 overlap 各段 during 必须一致的坑）/
  `tl.succession([...])`（逐段接续）+ `tl.add_group(Vec<Item>) -> Vec<ItemId>`（VGroup 注册侧）+
  `tl.save_state(id) -> Item` / `tl.restore(id, &snap) -> ItemId`（快照注册 + 完整 morph；
  返回新句柄，morph 后旧句柄退场）
- 验收：sugar_demo 场景（六段编排 8.1s/243 帧/15 动画）出片目检通过；together/succession/restore
  埋点时序核对一致；单测 8 项新增 + 集成 `sugar_薄糖批编排`（10 段断言）；SKILL.md 词汇表同步
- 教训：`(0..3).map(|i| ... i * 200.0)` 闭包变量被推断为 i32（E0277）——显式 `|i: i32|` + `f64::from(i)`

### M9.6 修复：encode 临时目录按 pid+调用序号隔离，根除同进程并发编码竞态（2026-10-12）

- fix(encode): 编码临时目录原只按进程 id 命名（`ffanim-encode-<pid>`）——同进程内并发
  `encode_video`（典型：cargo test 并行跑两个编码冒烟用例）共享 `seg_XXXX.mp4` 与
  `concat.txt`，先结束一方 `remove_dir_all` 删掉另一方正在写的段文件 → 并发偶发
  `concat 失败: No such file or directory`（ci-gpu 复发两次的根因）
- 修复：目录名追加进程内原子调用序号（`ffanim-encode-<pid>-<seq>`），pid 区分进程、
  序号区分同进程并发调用；属产品缺陷修复（用户多线程同时出片同样触发），非仅测试问题
- test: 新增 `并发编码互不干扰` 回归测试（双线程同时 encode_video 各自产物合法）；
  post-fix 循环 30 次全过（修复前第 2 次即复现）

### M9.5 manim 对齐第三批：强调动画四件套 indicate / circumscribe / flash / show_passing_flash（2026-10-12）

- feat(core): `Timeline::indicate(id, factor, dur)`——绕 bbox 中心放大 factor 再复原
  （manim Indicate 对应）：展开为两段 TransformTo（放大段 + 复原段），段后基线复位原位
- feat(core): `Timeline::circumscribe(id, dur, buff, color)`——目标 bbox 外扩 buff 的描边框
  淡入淡出圈选（manim Circumscribe 对应；临时对象靠 is_visible 的 appear/hidden_before 语义进出）
- feat(core): `Timeline::flash(at, dur, radius, color)`——at 处圆环淡入 + 扩张 1→2 倍 + 淡出
  三段同窗编排（manim Flash 对应，指向空白处）
- feat(core): `Timeline::show_passing_flash(id, t0, t1, dur, color)`——沿对象路径 [t0,t1] 段
  局部高亮描边（manim ShowPassingFlash 对应，需 Shape::Path）；配套 `span_path` 段级子路径提取
  （ClosePath 保留段显式 line_to 回子路径起点，避免错闭合）
- feat(core): 全部为构建期编排（既有词汇展开落表），求值路径零改动；四件套 +
  位姿解析分支 `log::info` 埋点齐全（峰值坐标/框范围/时序窗口，RUST_LOG=info 排查动画时序与坐标）
- feat(cli): emphasize_demo parity 场景（indicate 1.3 倍 / circumscribe 圈选 / flash 指空 /
  折线中段 passing flash，出片 5.9s 目检通过）
- test: span_path 单测 2 项 + 强调时序/坐标单测 4 项 + 集成测试 1 项（10 段展开断言）

### M9.4 manim 对齐第二批：相对布局 next_to / to_edge / arrange（2026-10-12）

- feat(core): 新增 layout 模块——`Item::next_to(&target, Direction, buff)`（邻接 + 另一轴中心对齐）/
  `Item::to_edge(Direction, buff, canvas)`（贴画布边，另一轴对画布居中，manim 帧边 next_to 语义）/
  `arrange(&mut items, Direction, buff) -> Rect`（依次排列，返回总包围盒供整组再平移）；
  全部构建期纯操作（bbox → 左乘平移仿射），`Direction` 视觉方向（屏幕 y 向下）
- feat(core): 布局核心分支 `log::info` 埋点——next_to/to_edge 四方向分支各打输入 bbox、
  对齐边计算式、平移量，函数尾打结果 bbox；arrange 打逐项与汇总（RUST_LOG=info 可查，构图排错用）
- feat(cli): layout_demo parity 场景（to_edge 标题贴顶 / arrange 横排居中 / arrange 竖排列表，出片 3.2s 目检通过）
- test: layout 单测 5 项（四方向邻接 / 贴边居中 / 批量总包围盒 / 不修改原对象）+ 集成测试 1 项；
  门面 re-export `Direction` / `arrange`

### M9.3 manim 对齐第一批：标注基元 + 位姿动作糖（2026-10-12）

- feat(core): 标注基元五件套——`Item::line` / `arrow`（线+闭合三角头，头长自适应 min(len/4, 32px)）/
  `dashed_line(a, b, dash, gap)` / `arc(center, r, 起角°, 止角°)`（视觉逆时针，manim 语义）/
  `ellipse(center, rx, ry)` + `Item::bbox()`（Path 控制点框 / Group 递归并集 / Image 按尺寸，含变换四角）
- feat(core): 位姿动作糖——`Anim::shift_to / move_to / rotate_to / scale_to`（manim `.animate.*` 对应）：
  构建期 `AnimKind::Pose` 中间表示 + `PoseDelta::resolve`（绕 bbox 中心的相对位姿 → 绝对仿射），
  `Timeline::play` 时解析落表为 `TransformTo`，编译表内不出现 Pose 变体（求值路径零改动）
- feat(cli): primitives_demo parity 场景（基元 group 描边生长 + 位姿动作串演，出片 5.2s 目检通过）
- test: item 基元/包围盒 6 项 + timeline 位姿语义 5 项 + 集成测试 1 项（PoseDelta 门面覆盖），
  SKILL.md 词汇表与 docs/05 P0-1/P0-4 状态同步

## [v0.2.0] - 2026-10-12

M7 地基与拆分 + 1080p 性能基线。semver 0.x 规则自本版本起生效。

### 新增

- justfile 任务编排（ci/ci-gpu/doc/demo/bench）+ CHANGELOG + README/LICENSE 重写 + docs/demo.gif（74403fe）
- missing_docs lint 全 workspace 生效 + 101 处公开项中文文档补全 + CI 加 cargo doc 门禁（7e73405）
- criterion bench（crates/ffanim-render/benches/render.rs）：cpu_render_1080p 26.43ms（NF1 实时率 1.21x 达标）、rgb24_pack_1080p 1.195ms（≈5.2GB/s）；数字入 docs/benchmarks.md（2465c25）

### 变更

- refactor(core): timeline 按职责拆分 anim/tracker/timeline/eval 四模块，公开 API 零变化（8343faf）
- refactor(cli): main.rs 按命令族拆分 cmd/ 六模块，--help 逐字不变（7d1da18）
- refactor(typst): lib.rs 拆分 engine/formula/matching 三模块，公开 API 零变化（cb727b5）
- docs: 项目优化规划 docs/04 + fourier 示例 BGM 来源与许可标注（b4603b7）
- fix: Formula::fit_transform 缩放居中修正，创作示例迁移至 examples/（ef7be8b）

## [v0.5-m5] - 2026-10-12

M5 Agent 工具链 + M6 创作验证。

### 新增

- ffanim tools：strip 接触表 + onion 洋葱皮，agent 视觉诊断工具链（643d233）
- ffanim-audio：audio analyze 诊断 + Duration::FromAudio 片长延展 + 音轨 mux（97bd2b4）
- ffanim new 脚手架 + 场景模板库 + 官方 coding-agent skill（3890213）
- M6 创作验证：傅里叶级数教学视频，实战倒推 Formula::fit_transform（fe7da28）
- 接入真实 BGM：Erik Satie《Gymnopédie No.1》公有领域录音（3c0ebdd）

## [v0.4-m4] - 2026-10-12

M4 坐标系与实时预览。

### 新增

- Axes 坐标系：c2p/p2c 双向映射 + 轴/刻度/网格 + 函数图像（Catmull-Rom 平滑）（f77c5fc）
- ValueTracker 值动画 + 求值期 updater 信号节点模型（a580a03）
- ffanim-preview 实时预览器（winit 0.30 + softbuffer，与 mp4 输出像素级一致）+ sine_demo 验收场景（f043bfa）

## [v0.3-m3] - 2026-10-12

M3 Typst 公式管线。

### 新增

- Typst 0.15 库嵌入：MathWorld + Frame→Items 适配 + 排版 LRU 缓存（51d88f4）
- 公式结构匹配 match_formulas（LCS 配对，TransformMatchingTex 基础）（19f0600）
- CLI inspect / snapshot 子命令（agent 排查与像素回归基准）+ formula_demo（e66af24）

## [0.1.0] - 2026-10-12

M0~M2：从骨架到第一支出片。

### 新增

- M0 workspace 骨架：8 crate 划分、主题数据驱动加载、CI（36f850c）
- M1 核心渲染链路：点阵模型 + CPU/GPU 双后端 + PNG 导出（58b138b）
- M2 时间轴两阶段模型（构建期记录 → 静态调度表 → 渲染期纯函数求值）+ ffmpeg 管道编码 + CLI render/timeline，渲染出第一支 mp4（f3b9f4b、f7ebb6c、6f472fa）
