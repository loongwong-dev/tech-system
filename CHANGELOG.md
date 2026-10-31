# Changelog

本文件记录使用者可感知的变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## 记录规则

- 只记录使用者可感知的变化：功能、行为、接口、配置、兼容性与安全修复。
- 使用者指产品用户、前端/集成方与运维部署方；仓库内部维护者不算。
- 不记录：文档重组、ADR、CI、Git hooks、内部重构、依赖升级（除非影响使用者）。
- 每次发版把 `Unreleased` 归档为 `## [x.y.z] - YYYY-MM-DD`，然后清空；同一变化只写一次。

## [Unreleased]

### Added

- API 骨架：按 `/auth`、`/api/v1`、`/admin/api/v1`、`/s` 分组的路由，RFC 9457 Problem Details，`/healthz` 与 `/readyz` 探针，请求 ID 与 tracing 中间件。
- Bun 前端 workspace：Vite + React + TanStack Router 的 web/admin 两个应用、共享 UI 与 API 类型包、Vitest 与首屏 JS 预算脚本。
- 邀请码注册：`POST /auth/register` 原子消耗邀请码，创建个人租户、owner 关系与 30 天会话，密码使用 Argon2id 哈希。
- 登录与会话：`POST /auth/login` 轮换会话、`POST /auth/logout` 立即吊销；会话 30 天滑动续期，每小时最多写一次。

### Fixed

- 收敛数据库应用角色权限：`app_tenant` 不再访问平台例外表与 `_sqlx_migrations`，`app_admin` 不再读写迁移历史；角色引导改为在迁移之后执行。

## [0.2.0] - 2026-10-31

> 项目基线建立期的一次性归档，包含内部文档与工具变更。后续版本只记录使用者可感知的变化。

### Added

- Apache License 2.0（`LICENSE`）。
- GitHub Actions 每日定时全历史 gitleaks 扫描。
- 文档问题与使用问题 Issue 表单（`docs.yml`、`question.yml`）。
- 文档治理与组织规范（`docs/governance.md`）：文档类型、所有权、评审、生命周期、季度审计、新项目落地路径与目录规模演进。
- 业务产品需求说明（`docs/product/platform/requirements.md`、`docs/product/feedback/requirements.md`）：教培教师 AI 反馈平台的 v1 范围、领域模型、核心链路、权益规则、管理端与合规要求。
- 六份架构决策记录：Rust 后端技术栈、PostgreSQL 自建任务队列、typst 渲染、多租户隔离与 RLS 双连接池、双前端与契约生成、文档分层（`docs/adr/0001`–`0006`）。
- 架构决策记录：C 端采用响应式 Web、暂不做小程序，登录用邮箱加密码（`docs/adr/0007-responsive-web-client.md`）。
- 四份技术设计文档：数据库设计、API 契约、认证与安全、Prompt 与质量（`docs/design/platform/database.md`、`docs/design/platform/api-contract.md`、`docs/design/platform/authentication.md`、`docs/design/feedback/prompt.md`）。
- 部署与测试文档：部署拓扑、备份恢复、发布回滚、可观测性与成本；必测清单、测试分层与 CI 分工（`docs/ops/deployment.md`、`docs/design/engineering/testing.md`）。
- 前端架构与冷启动计划：共享边界、离线草稿、微信兼容、性能预算；阶段推进、陪跑内测、邀请码节奏与内容获客（`docs/design/platform/frontend.md`、`docs/product/feedback/launch.md`）。
- 代码规范与文件规模门禁：工作区结构、依赖方向、模块分层、规模上限与豁免机制（`docs/design/engineering/code-standards.md`、`scripts/check-size.sh`）。
- Issue 驱动开发流程：三层任务结构、五态标签、AI 会话三检查点、追溯链与 `scripts/check-trace.sh`（`docs/process/issue-workflow.md`）。
- 任务与交付单元 Issue 模板（`.github/ISSUE_TEMPLATE/task.yml`、`epic.yml`）。
- 代码评审标准：八项评审维度、逻辑自洽三问、Conventional Comments 评论标签、规模控制与合并条件（`docs/process/code-review.md`，ADR-0011）；PR 模板同步增加验收证据、自检与评审结论。
- 文档归属决策：项目特有的代码与测试规范归入 `design/`（ADR-0013）；文档按性质与功能模块组织（ADR-0014）。
- ADR 生命周期决策：允许同主题原地修订，新增仅限新主题或被推翻的决策，一个 ADR 只记录一个决策或模块（ADR-0015）。
- 注释标准 CS-28：解释性注释使用简体中文；模块与公开接口必须有文档注释，关键逻辑必须有行内注释（`docs/design/engineering/code-standards.md` 第 11 节）。
- Lefthook 本地钩子：`pre-commit` 格式/规模/密钥，`commit-msg` 提交信息，`pre-push` 全量检查（`lefthook.yml`）。

### Changed

- `README.md` 由模板说明改为项目说明：定位、当前状态、v1 范围、技术栈、仓库结构、文档入口、门禁命令与已知限制。
- `docs/` 按性质重组：`process/` 放与业务无关的通用流程，`product/`、`design/`、`ops/` 放本项目内容；根目录只保留治理与运维文档；`check.sh` 支持目录级索引。
- ADR 规则调整：同主题优先修订并追加「修订记录」，仅新主题或被推翻的决策新增；`docs/adr/README.md` 增加「现行 / 历史」主题索引。
- ADR 整理：`0006`、`0012`、`0013` 标记 `Superseded by 0014`，`0014` 扩写为文档组织的唯一现行决策。
- 根目录文档按内容重新分类：`governance.md`、`ai-behavior.md` 为通用规范，`ci-cd.md`、`configuration.md` 为项目运维；删除模板迁移路径与未启用的可选工具说明。
- `configuration.md` 补充 CODEOWNERS 启用步骤与私有仓库的计划限制；Issue 表单、`AGENTS.md`、`CONTRIBUTING.md`、`SECURITY.md` 的模板身份措辞改为本仓库。
- 每份文档头部增加「类型」字段，区分通用规范、项目需求、项目设计与项目运维（ADR-0012），根目录文档与子目录 `README.md` 同步。
- `code-standards.md` 与 `testing.md` 从 `process/` 移至 `design/`，类型改为项目设计；`process/` 只保留 Issue 流程与代码评审（ADR-0013）。
- `product/` 与 `design/` 按模块重组：`platform/` 放跨模块规则与契约，`feedback/` 放当前模块；需求拆为平台需求与反馈模块需求，`prompt-design.md` 改为 `design/feedback/prompt.md`，`launch-plan.md` 改为 `feedback/launch.md`（ADR-0014）。
- 删除代码评审文档中对上一版评审方案的重复论证，评审标准只保留可执行方法；Issue 流程移除项目里程碑、模块标签与规格映射等专有取值。
- 代码评审「注释」维度增加中文与覆盖检查；代码规范评审清单同步 CS-28。
- 修正根目录文档中的过期路径与表单清单：`governance.md` 的新项目落地路径、`configuration.md` 的 Issue 表单列表。
- `check.sh` 新增 `size` 子命令并纳入 `all`，通过 `scripts/check-size.sh` 校验 Rust 与前端源码的文件行数上限。
- `check.sh` 新增 `commit-msg <file>` 子命令，供 Git `commit-msg` 钩子复用提交信息校验。
- `AGENTS.md` 新增 A7、A8：开发任务必须来自 Issue；AI 不得自行合并 PR，开 PR 前必须完成自检。
- Task 模板增加「完成定义」，与验收标准分开；Issue 流程补充「文档即规格」的映射关系。
- `check.sh` 新增文档文件名、ADR 索引、引用式链接校验；密钥输出改为脱敏；`CHECK_EXTERNAL_LINKS=1` 缺少 lychee 时失败。
- `check.sh` 支持 `%20` 本地链接，并检查 shell 脚本 Tab 与 Makefile 文本格式。
- GitHub Actions 取消 checkout 凭据持久化，并向 gitleaks 传递 `GITLEAKS_LICENSE`。
- 修正 Claude Code、Copilot 的指令文件支持说明与 gitleaks 扫描范围说明。
- 丰富 Bug 与改进建议表单：影响范围、版本、回归状态、环境、验收标准与补充信息。
- 提交标题支持可选的类型 emoji 前缀（如 `✨ feat(ci): ...`），`check.sh commits` 会校验 emoji 与 type 对应；兼容 Gitmoji 的 `⚡️`、`📦️`、`⏪️` 与破坏性变更 `💥`。

### Fixed

- 删除工作区遗留的空目录。
- 修正 `docs/engineering/` 残留引用：Issue 模板、根 `README.md` 目录树与 `CHANGELOG.md`。
- 修正模块化重组后的旧路径与索引：`docs/README.md`、`product/`、`design/`、ADR 与运维文档。
- `check.sh docs` 支持多级目录索引：模块子目录的 `README.md` 逐级登记，文档无需在 `docs/README.md` 逐篇列出。
- `check.sh size` 纳入 GitHub Actions，与本地 `all` 及门禁矩阵一致。

### Removed

- 移除 `.gitee/`、`.workflow/quality.yml` 及所有 Gitee Go 文档与配置，只维护 GitHub Actions 一套门禁。
- 移除 `skills/`、`scripts/install-skills.sh` 与 `docs/skills.md`；模板不再内置 Agent Skills 工作流。
- 模板基线中的示例 ADR 已移除；本项目的 ADR 从 `0001` 起记录。

## [0.1.0] - 2026-10-31

### Added

- 根级 `AGENTS.md`：AI 行为契约，覆盖上下文读取、修改、文档、提交、验证、安全与规则维护。
- `docs/` 文档体系：文档索引、AI 行为说明、CI/CD 说明、配置说明、ADR 流程与模板。
- `scripts/check.sh`：文档结构、站内链接、文本格式、Conventional Commits、密钥基线扫描。
- GitHub Actions 质量门禁：markdownlint、lychee、gitleaks，以及提交范围检查。
- 通用项目配置：`.editorconfig`、`.gitignore`、`.gitattributes`、`.markdownlint-cli2.jsonc`、`.lychee.toml`。
- GitHub Issue 与 PR 模板、Dependabot 配置。
- `README.md`、`CONTRIBUTING.md`、`SECURITY.md`。
