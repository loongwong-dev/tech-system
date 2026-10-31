# 项目配置说明

本仓库的配置分为必需基线与 GitHub 平台配置；CODEOWNERS 与 Lefthook 属于可选协作配置。

- **类型**：项目运维

## 必需基线

| 文件 | 职责 | 修改注意 |
| --- | --- | --- |
| `AGENTS.md` | AI 行为契约 | 改规则需同步 CHANGELOG 与 ADR |
| `LICENSE` | Apache License 2.0 | 修改分发时保留许可证与版权声明 |
| `.editorconfig` | 跨编辑器文本格式 | 与 `check.sh format` 保持一致 |
| `.gitattributes` | 换行与二进制标记 | 新增二进制类型时补充 |
| `.gitignore` | 排除本地环境、产物、密钥 | 不要忽略应共享的规则文件 |
| `.markdownlint-cli2.jsonc` | Markdown 规则 | 放宽规则需说明理由 |
| `.lychee.toml` | 链接检查范围与超时 | 排除项越少越好 |
| `scripts/check.sh` | 语言无关门禁 | 新增子命令需更新文档 |
| `.github/workflows/quality.yml` | GitHub 门禁 | Action 版本由 Dependabot 更新 |
| `.github/ISSUE_TEMPLATE/*` | Issue 表单 | 表单字段变化时同步 |
| `.github/pull_request_template.md` | GitHub PR 模板 | 与 CONTRIBUTING 要求保持一致 |
| `CONTRIBUTING.md` | 人的协作流程 | 流程变化时更新 |
| `CHANGELOG.md` | 使用者可感知的变更 | 合并前写 `Unreleased` |
| `SECURITY.md` | 泄露与漏洞报告方式 | 报告渠道：GitHub Security Advisories |
| `deploy/postgres/bootstrap.sql` | 数据库角色与权限引导 | 角色或权限模型变化时同步本文与数据库设计 |
| `package.json` / `bun.lock` | Bun workspace 清单与锁文件 | 第三方版本集中在根 `catalog`；工作区用 `catalog:`，内部包用 `workspace:*`；锁文件必须提交 |
| `bunfig.toml` | Bun 安装策略 | 固定 isolated linker，不混用 hoisted |
| `tsconfig.base.json` | 前端 TypeScript 基线 | 各应用通过 `extends` 继承 |
| `vitest.config.ts` | 前端测试范围 | 新增测试目录时同步 include |
| `.sqlx/` | SQLx 编译期查询缓存 | 由 `cargo sqlx prepare --workspace` 生成，必须提交 |
| `.cargo/config.toml` | SQLx 离线编译默认值 | 默认 `SQLX_OFFLINE=true`；刷新缓存时显式设为 `false` |

## 数据库连接（运行时）

| 环境变量 | 用途 | 使用者 |
| --- | --- | --- |
| `DATABASE_URL` | 表所有者连接，执行迁移与角色引导 | `sqlx migrate run --source crates/db/migrations`、`deploy/postgres/bootstrap.sql` |
| `DATABASE_URL_TENANT` | `app_tenant` 角色连接，受 RLS 约束 | api 租户池 |
| `DATABASE_URL_ADMIN` | `app_admin` 角色连接，具备 `BYPASSRLS` | api 与 worker 管理池 |
| `API_ADDR` | API 监听地址，默认 `0.0.0.0:8080` | `cargo run -p api` |
| `WORKER_ID` | worker 标识，默认 `worker-<pid>` | `cargo run -p jobs --bin worker` |
| `WORKER_POLL_SECONDS` | 空队列轮询间隔，默认 1 | worker |
| `WORKER_LEASE_SECONDS` | 任务租约，默认 120 | worker |
| `WORKER_RECLAIM_SECONDS` | 过期租约扫描间隔，默认 30 | worker |
| `RUST_LOG` | `tracing` 过滤级别，默认 `info` | api 与 worker |

- 连接池上限：租户池 10，管理池 5，定义见 `crates/db/src/config.rs`。
- 首次部署顺序：先执行 `sqlx migrate run --source crates/db/migrations`，再执行 `deploy/postgres/bootstrap.sql` 创建角色、授权并撤销例外表权限。
- 每次新增平台例外表后必须重跑 `bootstrap.sql`，否则默认权限会把新表授予 `app_tenant`。
- 角色登录凭据由部署环境单独配置，不写入仓库。

## GitHub

- `.github/workflows/quality.yml`：五个 job（docs、rust、frontend、links、secrets）。
- `.github/dependabot.yml`：每月检查 GitHub Actions 版本。
- Issue 表单：Bug（`bug.yml`）、改进建议（`feature.yml`）、文档问题（`docs.yml`）、使用问题（`question.yml`）、开发任务（`task.yml`）、交付单元（`epic.yml`）；`config.yml` 关闭空白 Issue。
- Issue 表单使用 YAML form，包含影响范围、环境、复现/验收等必填项。
- PR 模板路径：`.github/pull_request_template.md`。

## Git hooks（Lefthook）

`lefthook.yml` 定义本地钩子；CI 不依赖它。

安装：

```bash
brew install lefthook
lefthook install
```

其他平台见 <https://lefthook.dev/install/>。

| 钩子 | 执行内容 |
| --- | --- |
| `pre-commit` | `check.sh format`、`check.sh size`、`check.sh secrets`，并行执行 |
| `commit-msg` | `check.sh commit-msg {1}` 校验 Conventional Commits 与 emoji |
| `pre-push` | `check.sh all` |

规则：

- 钩子只做检查，不自动修改文件；格式化修复由人显式执行。
- 每次新克隆后运行 `lefthook install`；新增钩子类型后重新运行。
- 手动执行：`lefthook run pre-commit --all-files`。
- `git commit --no-verify` / `git push --no-verify` 可以绕过钩子；CI 仍是最终门禁。

## CODEOWNERS

当前为单人维护，`.github/CODEOWNERS` 未启用。单人账号不会给自己的 PR 发评审请求，建议加入第二位评审者后再启用。需要启用时：

1. 在默认分支创建 `.github/CODEOWNERS`，也支持根目录或 `docs/CODEOWNERS`。
2. 写入所有者。最后匹配的规则生效：

   ```text
   * @loongwong-dev
   ```

3. 被指定的账号必须对仓库有写权限；团队使用 `@org/team`。
4. 提交并推送到默认分支 `main`。CODEOWNERS 只从默认分支读取。
5. 在 GitHub 仓库 Settings → Branches / Rules 中启用：
   - Require a pull request before merging；
   - Require review from Code Owners。
6. 验证：开一个修改 `docs/` 的 PR，确认自动请求 `@loongwong-dev` 评审，且未批准时不能合并。

> 分支保护与仓库规则集对**私有仓库**需要 GitHub Pro；免费账号的私有仓库会返回 403（“Upgrade to GitHub Pro or make this repository public”）。当前仓库为私有，启用前需升级 GitHub Pro、改为公开仓库，或迁移到 Team/Enterprise 组织。

个人仓库不能自我批准。CODEOWNERS 只负责自动请求评审，实际合并仍按 [ADR-0010](adr/0010-mandatory-review.md) 由维护者人工确认。

## 版本与发布

本仓库使用语义化版本，但**不自动发版**：

1. 按 [CHANGELOG 记录规则](../CHANGELOG.md) 在 `Unreleased` 下记录使用者可感知的变更。
2. 需要发布时，把 `Unreleased` 改为 `## [x.y.z] - YYYY-MM-DD`。
3. 打 tag：`vX.Y.Z`。
4. 不强制 GitHub Release；需要时再创建。

## 维护节奏

| 频率 | 动作 |
| --- | --- |
| 每次变更 | `bash scripts/check.sh all` |
| 每月 | 合并 Dependabot 的 Actions 更新；抽查外链 |
| 每季度 | 审计 `AGENTS.md` 规则是否仍可执行；清理过期排除项；按 [文档治理](governance.md) 审计文档 |
| 平台变更时 | 复核 GitHub Actions 语法与权限 |

文档维护与审计细节见 [文档治理与组织](governance.md)。

## 参考

- EditorConfig：<https://editorconfig.org/>
- Git attributes：<https://git-scm.com/docs/gitattributes>
- Conventional Commits：<https://www.conventionalcommits.org/zh-hans/v1.0.0/>
- Keep a Changelog：<https://keepachangelog.com/zh-CN/1.1.0/>
- ADR：<https://adr.github.io/>
- GitHub CODEOWNERS：<https://docs.github.com/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners>

访问日期：2026-10-30。
