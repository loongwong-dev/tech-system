# Changelog

本文件记录使用者可感知的变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Added

- Apache License 2.0（`LICENSE`）。
- GitHub Actions 每日定时全历史 gitleaks 扫描。
- 文档问题与使用问题 Issue 表单（`docs.yml`、`question.yml`）。
- 文档治理与组织规范（`docs/governance.md`）：文档类型、所有权、评审、生命周期、季度审计、新项目落地路径与目录规模演进。

### Changed

- `check.sh` 新增文档文件名、ADR 索引、引用式链接校验；密钥输出改为脱敏；`CHECK_EXTERNAL_LINKS=1` 缺少 lychee 时失败。
- `check.sh` 支持 `%20` 本地链接，并检查 shell 脚本 Tab 与 Makefile 文本格式。
- GitHub Actions 取消 checkout 凭据持久化，并向 gitleaks 传递 `GITLEAKS_LICENSE`。
- 修正 Claude Code、Copilot 的指令文件支持说明与 gitleaks 扫描范围说明。
- 丰富 Bug 与改进建议表单：影响范围、版本、回归状态、环境、验收标准与补充信息。
- 提交标题支持可选的类型 emoji 前缀（如 `✨ feat(ci): ...`），`check.sh commits` 会校验 emoji 与 type 对应；兼容 Gitmoji 的 `⚡️`、`📦️`、`⏪️` 与破坏性变更 `💥`。

### Fixed

- 删除工作区遗留的空目录。

### Removed

- 移除 `.gitee/`、`.workflow/quality.yml` 及所有 Gitee Go 文档与配置，只维护 GitHub Actions 一套门禁。
- 移除 `skills/`、`scripts/install-skills.sh` 与 `docs/skills.md`；模板不再内置 Agent Skills 工作流。
- 模板内置的 `ADR-0001`～`ADR-0004`；`docs/adr/` 仅保留流程与模板，派生项目从 `0001` 开始记录自己的决策。

## [0.1.0] - 2026-10-31

### Added

- 根级 `AGENTS.md`：AI 行为契约，覆盖上下文读取、修改、文档、提交、验证、安全与规则维护。
- `docs/` 文档体系：文档索引、AI 行为说明、CI/CD 说明、配置说明、ADR 流程与模板。
- `scripts/check.sh`：文档结构、站内链接、文本格式、Conventional Commits、密钥基线扫描。
- GitHub Actions 质量门禁：markdownlint、lychee、gitleaks，以及提交范围检查。
- 通用项目配置：`.editorconfig`、`.gitignore`、`.gitattributes`、`.markdownlint-cli2.jsonc`、`.lychee.toml`。
- GitHub Issue 与 PR 模板、Dependabot 配置。
- `README.md`、`CONTRIBUTING.md`、`SECURITY.md`。
