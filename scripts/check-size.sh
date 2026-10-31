#!/usr/bin/env bash
# 文件规模检查：Rust 与前端源码超过硬上限即失败。
#
# 统计规则：
#   - Rust：空行不计，遇到 #[cfg(test)] 后停止统计（测试代码不参与上限）。
#   - 前端：空行不计，跳过测试文件与生成物。
#   - 首 5 行含 "code-size:allow" 的文件跳过，用于确实无法拆分的例外。
#
# 用法：bash scripts/check-size.sh
# 环境变量：MAX_LINES（默认 500）

set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT" || exit 1

MAX="${MAX_LINES:-500}"
fail=0
checked=0

has_waiver() {
  head -n 5 "$1" 2>/dev/null | grep -q 'code-size:allow'
}

count_rust() {
  awk '
    /^[[:space:]]*#\[cfg\(test\)\]/ { exit }
    NF { n++ }
    END { print n + 0 }
  ' "$1"
}

count_text() {
  awk 'NF { n++ } END { print n + 0 }' "$1"
}

check_file() {
  local f="$1" n
  has_waiver "$f" && return 0
  n="$(count_rust "$f")"
  checked=$((checked + 1))
  if [ "$n" -gt "$MAX" ]; then
    printf 'ERROR: %s: %s lines exceeds %s (add // code-size:allow with a reason if unavoidable)\n' "$f" "$n" "$MAX" >&2
    fail=1
  fi
}

check_text_file() {
  local f="$1" n
  has_waiver "$f" && return 0
  n="$(count_text "$f")"
  checked=$((checked + 1))
  if [ "$n" -gt "$MAX" ]; then
    printf 'ERROR: %s: %s lines exceeds %s\n' "$f" "$n" "$MAX" >&2
    fail=1
  fi
}

if [ -d crates ]; then
  while IFS= read -r f; do
    check_file "$f"
  done < <(find crates -type f -name '*.rs' -not -path '*/target/*' -not -path '*/tests/*' | sort)
fi

for base in apps packages; do
  [ -d "$base" ] || continue
  while IFS= read -r f; do
    case "$f" in
      *.test.ts|*.test.tsx|*.spec.ts|*.spec.tsx|*.d.ts) continue ;;
      */api-types/*) continue ;;
      */e2e/*) continue ;;
    esac
    check_text_file "$f"
  done < <(find "$base" -type f \( -name '*.ts' -o -name '*.tsx' \) -not -path '*/node_modules/*' -not -path '*/dist/*' | sort)
done

if [ "$fail" -ne 0 ]; then
  printf '\nFile size check failed.\n' >&2
  exit 1
fi

printf 'File size check passed (%s files, limit %s lines).\n' "$checked" "$MAX"
