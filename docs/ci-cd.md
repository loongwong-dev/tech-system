# CI/CD 与质量门禁

目标不是“有一个绿勾”，而是让**可机检的规范真的被检查**，并且本地与 GitHub Actions 的结论尽量一致。

- **类型**：项目运维

## 门禁矩阵

| 检查 | 本地 | GitHub Actions | 失败时先看 |
| --- | --- | --- | --- |
| Markdown 规则 | `npx markdownlint-cli2`（可选） | `markdownlint-cli2-action` | `.markdownlint-cli2.jsonc` |
| 文档结构 / 文件名 / H1 / 索引 / ADR | `check.sh docs` | `check.sh docs` | 缺失文件、非法文件名或未登记索引 |
| 站内链接 | `check.sh docs` | `check.sh docs` + lychee | 相对路径 |
| 外链 | `CHECK_EXTERNAL_LINKS=1 check.sh links` | lychee | 目标站点状态、排除项 |
| 密钥扫描 | `check.sh secrets`（基线） | gitleaks | 泄露位置、是否需轮换 |
| 提交信息 | `check.sh commits` | 按 PR/推送范围检查 | Conventional Commits 格式 |
| 提交 Issue 引用 | `BASE_SHA=<ref> check-trace.sh` | PR / push 范围检查 | 提交 subject 或 body 缺少 `#N` |
| PR 关联 Issue | 人工检查 | PR 正文必须含 `Closes #N` | PR 正文 |
| Rust 格式 | `cargo fmt --all --check` | `rust` job | 格式差异 |
| Rust lint / 测试 | `cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` | `rust` job + PostgreSQL service | 编译、lint、集成测试 |
| SQLx 查询缓存 | `cargo sqlx prepare --workspace --check` | `rust` job | `.sqlx` 与代码/数据库不一致 |
| OpenAPI 契约 | `bash scripts/check-openapi.sh` | `contract` job | `openapi.json`、`schema.d.ts` 与 utoipa 注解漂移 |
| 前端安装 / 类型 / 单测 / 构建 / 预算 | `bun install --frozen-lockfile`、`bun run typecheck/test/build/size` | `frontend` job | 锁文件、类型、测试、构建或首屏预算 |
| 源码文件规模 | `check.sh size` | `check.sh size` | 文件行数上限、豁免注释 |
| 文本格式 | `check.sh format` | `check.sh format` | CRLF、Tab、末尾换行 |
| 依赖更新 | 手动 | Dependabot（每月） | Actions 版本 |

> “基线”表示轻量正则扫描，不能替代 gitleaks；GitHub 端已启用 gitleaks。

## 本地运行

```bash
bash scripts/check.sh all
```

分项运行：

```bash
bash scripts/check.sh docs
bash scripts/check.sh format
bash scripts/check.sh commits
bash scripts/check.sh secrets
CHECK_EXTERNAL_LINKS=1 bash scripts/check.sh links

BASE_SHA=origin/main bash scripts/check-trace.sh

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo sqlx prepare --workspace --check
bash scripts/check-openapi.sh

bun install --frozen-lockfile
bun run typecheck
bun run test
bun run build
bun run size
```

`CHECK_EXTERNAL_LINKS=1` 需要本机已安装 `lychee`；缺失时命令会失败，而不是静默跳过。

`check.sh` 只依赖 bash、git 与 POSIX 常用文本工具（grep、awk、sed、find、sort 等）。`all` 会在检测到 `markdownlint-cli2` 时额外运行完整 Markdown 规则；未安装时跳过。需要完整规则时运行：

```bash
npx markdownlint-cli2 "**/*.md"
```

## GitHub Actions

文件：`.github/workflows/quality.yml`。

- `docs`：checkout 全历史 → markdownlint → `check.sh docs/format/commits/size` → `check-trace.sh` → PR 正文 `Closes #N` 检查。
- `rust`：启动 PostgreSQL 17 service → 按 `rust-toolchain.toml` 安装工具链 → 安装 sqlx-cli → 执行迁移 → `cargo fmt/clippy/test` → `SQLX_OFFLINE=false cargo sqlx prepare --workspace --check`。
- `frontend`：安装 Bun 1.4.4 → `bun install --frozen-lockfile` → `typecheck/test/build/size`。
- `contract`：安装 Rust 与 Bun → 重新生成 `openapi.json` 与 `packages/api-types/src/schema.d.ts` → 与提交物比对，漂移即失败。
- `links`：lychee 按 `.lychee.toml` 检查外链与站内链接。
- `secrets`：`check.sh secrets` 基线扫描 + gitleaks；push/PR 扫描事件范围，定时/手动触发扫描完整历史。
- 触发：推送到 `main`、所有 PR、手动触发、每日定时扫描。
- 权限：默认 `contents: read`；gitleaks job 需要 `pull-requests: read` 来读取 PR 提交列表，关闭 PR 评论以避免申请写权限。
- 组织账号使用 gitleaks 需要免费 license，并配置 `GITLEAKS_LICENSE` secret；workflow 已将该 secret 传给 action。个人账号不需要。
- fork PR 不接收仓库 secret，组织账号的 fork PR 会因缺少 license 失败；可改用基线扫描或自建 Runner。
- 并发：同一分支的新运行会取消旧运行。

提交范围通过 `BASE_SHA` / `HEAD_SHA` 传入：

- PR：base SHA..head SHA。
- push：`github.event.before..github.sha`；首次推送的 before 全零时退化为只查 HEAD。

> gitleaks-action 的 PR 扫描只取 GitHub API 返回的一页提交（默认 30 个，未分页）；大 PR 末尾的提交可能未覆盖。需要完整覆盖时使用 `workflow_dispatch` 或每日 `schedule`。

## 分支保护

在 GitHub 仓库 Settings → Branches / Rules 为 `main` 开启：

1. Require a pull request before merging。
2. Require status checks to pass before merging，必选：
   - `Docs and commits`
   - `Rust`
   - `Frontend`
   - `Links`
   - `Secrets`
   - `OpenAPI contract`
3. Require conversation resolution before merging。
4. 禁止直接 push 到 `main`；合并只能由维护者执行。

检查名称来自 `.github/workflows/quality.yml` 的 job `name`。修改 job 名称时必须同步本文件，否则分支保护会等待一个不存在的检查。

> **当前状态（2026-10-31）**：仓库已转为公开，`main` 分支保护已启用。
> 必选检查为 `Docs and commits`、`Rust`、`Frontend`、`Links`、`Secrets`；
> 管理员同样受保护，禁止 force push 与删除分支。
> 若以后转回私有且未订阅 GitHub Pro，分支保护将不可用。

私有仓库启用分支保护可能需要 GitHub Pro；免费账号的限制见[项目配置说明](configuration.md#codeowners)。

## 新增一个门禁

1. 先判断能否用现有 `check.sh` 子命令完成；能则不加新工具。
2. 需要新工具时，优先选单二进制、无运行时依赖、可在 GitHub hosted runner 运行的工具。
3. 在 `scripts/check.sh` 增加子命令，并保证失败时退出码非零。
4. 同步更新：本文件矩阵、`docs/configuration.md`、`README.md` 的检查说明。
5. 若引入新依赖或 CI 能力，补 ADR。

## 排障顺序

1. 本地复现：`bash scripts/check.sh <子命令>`。
2. 看首个错误，不要先改配置放宽规则。
3. 链接失败：确认是否为 403/429、是否需要排除、是否应改为官方稳定链接。
4. 密钥失败：先确认是否真实泄露；真实泄露需轮换，而不是只删文件。
5. CI 与本地差异：对比工具版本、工作目录、换行设置。

## 参考

- GitHub Actions 文档：<https://docs.github.com/en/actions>
- markdownlint-cli2：<https://github.com/DavidAnson/markdownlint-cli2>
- lychee：<https://lychee.cli.rs/>
- gitleaks：<https://github.com/gitleaks/gitleaks>

访问日期：2026-10-30。
