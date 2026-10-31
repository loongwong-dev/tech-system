#!/usr/bin/env bash
# 校验提交是否引用 Issue（#N）。
#
# 用法：
#   BASE_SHA=<ref> [HEAD_SHA=<ref>] bash scripts/check-trace.sh
#
# 默认检查 BASE_SHA..HEAD。合并提交与 fixup/squash 提交跳过。
# 仓库初始化阶段的提交没有 Issue 可引用，因此本脚本不纳入 check.sh all。

set -u

BASE="${BASE_SHA:-}"
HEAD_REF="${HEAD_SHA:-HEAD}"

if [ -z "$BASE" ]; then
  printf 'error: BASE_SHA is required (e.g. BASE_SHA=origin/main)\n' >&2
  exit 2
fi

if ! git rev-parse --verify --quiet "$BASE^{commit}" >/dev/null 2>&1; then
  printf 'error: BASE_SHA is not a commit: %s\n' "$BASE" >&2
  exit 2
fi

fail=0
count=0

while IFS= read -r sha; do
  [ -n "$sha" ] || continue
  subject="$(git log -1 --format='%s' "$sha")"
  case "$subject" in
    'Merge '*|'fixup! '*|'squash! '*) continue ;;
  esac

  body="$(git log -1 --format='%B' "$sha")"
  count=$((count + 1))

  if printf '%s' "$body" | grep -qE '(^|[^0-9A-Za-z])#[0-9]+'; then
    continue
  fi

  printf 'ERROR: %s lacks an issue reference (#N): %s\n' "${sha:0:8}" "$subject" >&2
  fail=1
done < <(git rev-list "$BASE..$HEAD_REF" 2>/dev/null)

if [ "$fail" -ne 0 ]; then
  printf '\nTraceability check failed. Add (#N) to the subject or "Refs #N" to the body.\n' >&2
  exit 1
fi

printf 'Traceability check passed (%s commit(s)).\n' "$count"
