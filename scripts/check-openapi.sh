#!/usr/bin/env bash
# 校验提交的 openapi.json 与前端类型是否落后于 utoipa 注解。
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

cargo run --quiet -p api --bin openapi > "$tmpdir/openapi.json"
diff -u openapi.json "$tmpdir/openapi.json"

bunx openapi-typescript "$tmpdir/openapi.json" -o "$tmpdir/schema.d.ts"
diff -u packages/api-types/src/schema.d.ts "$tmpdir/schema.d.ts"
