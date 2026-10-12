#!/usr/bin/env bash
# API 覆盖率门禁（docs/02 规则 B）：门面 re-export / pub 项必须在 examples 或 tests 中被引用。
#
# 用法：scripts/api_coverage.sh
# 退出码：0 = 全覆盖；1 = 有 API 无引用（打印缺口清单）。
set -euo pipefail
cd "$(dirname "$0")/.."

FACADE=crates/ffanim/src/lib.rs
# 引用扫描范围：示例 + 集成测试 + 基准
SITES="crates/ffanim/examples crates/ffanim/tests"

# 白名单：基础设施型导出（由内部测试/下游用户间接覆盖，不要求示例直接引用）
WHITELIST='^(Result|Error)$'

# 1) 提取门面公开名单：pub use / pub fn / pub const / pub mod 的导出标识符
names=$({
  # pub use a::{X, Y}; / pub use a::X; / pub use a::{...} 跨行块
  perl -0777 -ne 'while (/pub use [^;]*?\{([^}]*)\}/g) { print "$1\n" }
                  while (/pub use \w+::(\w+)\s*;/g) { print "$1\n" }' "$FACADE"
  grep -oP 'pub (?:fn|const|mod) \K\w+' "$FACADE"
} | tr ',' '\n' | sed 's/^[ \t]*//;s/[ \t]*$//' | grep -v '^$' | sort -u)

total=$(echo "$names" | wc -l)
missing=0
echo "== API 覆盖率（规则 B：门面 pub API 须被 examples/tests 引用）=="
for n in $names; do
  if echo "$n" | grep -qE "$WHITELIST"; then
    printf '  %-24s SKIP (白名单)\n' "$n"
    continue
  fi
  if grep -rqw "$n" $SITES 2>/dev/null; then
    printf '  %-24s ok\n' "$n"
  else
    printf '  %-24s MISSING\n' "$n"
    missing=$((missing + 1))
  fi
done

echo "== 合计 $total 个公开项，缺失 $missing =="
[ "$missing" -eq 0 ]
