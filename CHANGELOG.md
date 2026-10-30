# Changelog

本文件记录使用者可感知的变更。格式基于 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，版本遵循 [Semantic Versioning](https://semver.org/lang/zh-CN/)。

## [Unreleased]

### Added

- Apache License 2.0（`LICENSE`）。

## [0.1.0] - 2026-10-31

### Added

- 根级 `AGENTS.md`：AI 行为契约，覆盖上下文读取、修改、文档、提交、验证、安全与规则维护。
- `docs/` 文档体系：文档索引、AI 行为说明、CI/CD 说明、配置说明、Agent Skills 说明、ADR 流程与模板。
- `scripts/check.sh`：文档结构、站内链接、技能元数据、文本格式、Conventional Commits、密钥基线扫描。
- GitHub Actions 质量门禁：markdownlint、lychee、gitleaks，以及提交范围检查。
- Gitee Go 流水线：语言无关的 `check.sh all`。
- Agent Skills 工作流：`clarify`、`research`、`spec`、`implement`、`review`、`handoff`、`retro`。
- `scripts/install-skills.sh`：将技能链接到 `~/.claude/skills` 与 `~/.agents/skills`。
- 通用项目配置：`.editorconfig`、`.gitignore`、`.gitattributes`、`.markdownlint-cli2.jsonc`、`.lychee.toml`。
- GitHub/Gitee Issue 与 PR 模板、Dependabot 配置。
- `README.md`、`CONTRIBUTING.md`、`SECURITY.md`。

### Decisions

- [ADR-0001](docs/adr/0001-agents-md-single-source.md)：以 `AGENTS.md` 作为单一行为契约。
- [ADR-0002](docs/adr/0002-language-agnostic-shell-gates.md)：核心门禁使用语言无关的 Shell。
- [ADR-0003](docs/adr/0003-dual-platform-ci.md)：GitHub 为主、Gitee 对等的双平台 CI。
- [ADR-0004](docs/adr/0004-agent-skills-workflow.md)：用 Agent Skills 承载工作流。
