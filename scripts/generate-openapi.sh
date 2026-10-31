#!/usr/bin/env bash
# 从 utoipa 注解生成 openapi.json 与前端类型。
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

cargo run --quiet -p api --bin openapi > openapi.json
bunx openapi-typescript openapi.json -o packages/api-types/src/schema.d.ts
