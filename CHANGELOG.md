# 更新日志

本文件记录 ffanim 所有显著变更，格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
版本号自 v0.2.0 起执行 semver 0.x 规则（见 docs/04-项目优化规划.md §3.3）。

## [Unreleased]

### Added

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

### 新增（M8：功能补齐，进行中）

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
