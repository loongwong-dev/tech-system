set dotenv-load

# 查看可用任务
default:
    @just --list

# 安装前端依赖
install:
    bun install --frozen-lockfile

# 执行数据库迁移；DATABASE_URL 来自根 .env
migrate:
    sqlx migrate run --source crates/db/migrations

# 创建或刷新数据库应用角色与权限
bootstrap:
    psql "$DATABASE_URL" -f deploy/postgres/bootstrap.sql

# 启动 API
api:
    cargo run -p api --bin api

# 启动 worker
worker:
    cargo run -p jobs --bin worker

# 启动 C 端；Vite 读取 apps/web/.env.local
web:
    bun run --filter '@tech-system/web' dev

# Rust 格式化
fmt:
    cargo fmt --all

# 检查 Rust 格式化
fmt-check:
    cargo fmt --all --check

# Rust lint
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Rust 测试
test-rust:
    cargo test --workspace

# 前端测试
test-web:
    bun run test

# 全量测试
test:
    cargo test --workspace
    bun run test

# 前端类型检查
typecheck:
    bun run typecheck

# 构建两个前端
build:
    bun run build

# 首屏预算检查
size:
    bun run size

# 生成 OpenAPI 与前端类型
contract:
    bash scripts/generate-openapi.sh

# 检查 OpenAPI 生成物漂移
contract-check:
    bash scripts/check-openapi.sh

# 安装 Lefthook 本地钩子
hooks:
    lefthook install

# 刷新 SQLx 查询缓存
prepare:
    SQLX_OFFLINE=false cargo sqlx prepare --workspace

# 仓库文档、格式、提交与密钥检查
check:
    bash scripts/check.sh all

# 本地执行完整 CI 序列
ci:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace
    bun run typecheck
    bun run test
    bun run build
    bun run size
    bash scripts/check.sh all
    bash scripts/check-openapi.sh
