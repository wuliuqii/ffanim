#!/usr/bin/env bash
# 性能回归门禁（docs/07 §6.1）：bench 各维度对阈值断言，>50% 硬回退即红。
#
# 阈值 = docs/benchmarks.md 基线 × 宽松系数（编译/求值 ×2~3，渲染/读回 ×1.5~3，
# 防环境波动误报；>20% 软回退仍走 benchmarks.md 的人工说明条款）。
#
# 用法：just bench-gate（发版前 / 依赖升级后手动跑；CI 不跑——太慢）
# 退出码：0 = 全部达标；1 = 有指标超阈（打印明细）。
set -euo pipefail
cd "$(dirname "$0")/.."

# 阈值表（µs）："基准名:上限"。来源 docs/benchmarks.md：
#   compile_60_items 10.5µs / eval_frame_1080p 3.5µs（2026-10-15）
#   cpu_render_1080p 35.73ms / gpu_render_1080p 2.73ms / rgb24_pack_1080p 1.43ms
GATES=(
  "compile_60_items:30"
  "eval_frame_1080p:10"
  "cpu_render_1080p:54000"
  "gpu_render_1080p:8000"
  "rgb24_pack_1080p:3000"
)

echo "== 跑基准（criterion，约 1 分钟）=="
cargo bench -p ffanim-core --bench pipeline >/dev/null
cargo bench -p ffanim-render --bench render >/dev/null

echo "== 阈值断言 =="
fail=0
for gate in "${GATES[@]}"; do
  name="${gate%%:*}"
  limit="${gate##*:}"
  est="target/criterion/$name/new/estimates.json"
  if [ ! -f "$est" ]; then
    echo "  SKIP  $name（无结果，如 GPU 不可用）"
    continue
  fi
  # mean.point_estimate 单位 ns → µs 取整
  us=$(python3 -c "import json,sys;print(int(json.load(open('$est'))['mean']['point_estimate']/1000))")
  if [ "$us" -le "$limit" ]; then
    printf '  PASS  %-22s %8d µs  <= %d µs\n' "$name" "$us" "$limit"
  else
    printf '  FAIL  %-22s %8d µs  >  %d µs\n' "$name" "$us" "$limit"
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "性能门禁未过：存在 >50% 回退，核对 docs/benchmarks.md 后修复或说明。"
  exit 1
fi
echo "性能门禁全绿"
