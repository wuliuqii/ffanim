# 更新日志

本文件记录 ffanim 所有显著变更，格式遵循 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)。
版本号自 v0.2.0 起执行 semver 0.x 规则（见 docs/04-项目优化规划.md §3.3）。

## [Unreleased]

### Added

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
