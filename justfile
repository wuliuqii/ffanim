# ffanim 任务编排（docs/04-项目优化规划.md §5.1）
# 用法：just <配方名>；不带参数列出全部配方

default:
	@just --list

# 质量门禁：与 GitHub Actions 完全对齐（fmt + clippy + test + API 覆盖率）
ci:
	cargo fmt --all --check
	cargo clippy --workspace --all-targets -- -D warnings
	cargo test --workspace
	./scripts/api_coverage.sh
	@echo "CI 门禁全绿"

# GPU 特性编译验证（CI build-gpu job 对应；无 GPU 时运行期测试自动跳过）
ci-gpu:
	cargo clippy --workspace --all-targets --features ffanim-render/gpu -- -D warnings
	cargo test --workspace --features ffanim-render/gpu

# rustdoc 零警告门禁（docs/04 §3.4 规则 A）
doc:
	cargo doc --workspace --no-deps
	@echo "rustdoc 零警告"

# 官方示例：basic 族场景出片（argv 寻址，缺省全渲染；对齐 manim example_scenes）
demo:
        cargo run --release -p ffanim --example basic -- demo_anim

# 公式示例：typography 族场景（Typst 排版 + 结构匹配变换）
demo-formula:
        cargo run --release -p ffanim --example typography -- formula_demo

# 创作示例：傅里叶级数教学视频（BGM_PATH 环境变量可选混流音轨）
fourier:
	cargo run --release -p ffanim --example fourier

# 性能基准：criterion（1080p 单帧渲染基线，数字入库 docs/benchmarks.md）
bench:
	cargo bench -p ffanim-render
