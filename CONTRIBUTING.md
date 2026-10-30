# 贡献指南

本模板优先服务个人使用，但流程按“可复用、可审查”设计。以下规则对人和 AI 代理同样适用；AI 的额外约束见 [AGENTS.md](AGENTS.md)。

## 开始之前

1. 阅读 [README.md](README.md) 与 [docs/README.md](docs/README.md)。
2. 确认改动属于哪一类：规则、文档、CI、模板、配置。
3. 涉及工具/平台/目录结构的长期选择，先写 ADR（见 [docs/adr/README.md](docs/adr/README.md)）。

## 分支与提交

- 主分支：`main`。
- 分支名：`feat/<topic>`、`fix/<topic>`、`docs/<topic>`、`ci/<topic>`、`chore/<topic>`。
- 提交信息遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/v1.0.0/)。

格式：

```text
<type>(<scope>): <description>

[optional body]

[optional footer]
```

允许的 `type`：

| type | 用途 |
| --- | --- |
| `feat` | 新增规则、检查、模板能力 |
| `fix` | 修复错误、失效链接、错误配置 |
| `docs` | 仅文档 |
| `style` | 格式调整，不改变行为 |
| `refactor` | 重构，不改变外部行为 |
| `perf` | 性能优化 |
| `test` | 测试相关 |
| `build` | 构建系统或依赖 |
| `ci` | CI/CD 配置 |
| `chore` | 杂项维护 |
| `revert` | 回滚 |

约束：

- 标题 ≤ 72 字符（硬上限 100），不加句号。
- 一个提交只做一件事。
- scope 仅允许小写字母、数字和 `.`、`_`、`-`、`/`；也可以省略 scope。
- 破坏性变更使用 `!` 或 `BREAKING CHANGE:`。

示例：

```text
feat(ci): add gitee go quality pipeline
fix(docs): repair broken ADR link
docs(agents): clarify conflict precedence
```

## 本地检查

提交前运行：

```bash
bash scripts/check.sh all
```

需要完整 Markdown 规则时：

```bash
npx markdownlint-cli2 "**/*.md"
```

检查失败时修根因，不要删除规则、跳过检查或使用 `--no-verify`。

## 文档变更

- 新增 `docs/*.md` 必须登记到 [docs/README.md](docs/README.md)。
- 每个文档只有一个 H1，文件名小写 kebab-case。
- 站内链接用相对路径，外链用完整 URL。
- 行为或配置变化必须同步更新对应说明。
- 使用者可感知的变化写入 [CHANGELOG.md](CHANGELOG.md) 的 `Unreleased`。

## PR 要求

PR 描述必须包含：

1. **改了什么**：文件与行为。
2. **为什么**：问题、约束、取舍。
3. **如何验证**：实际运行的命令与结果。
4. **风险与回滚**：可能影响的范围。
5. **关联 Issue**：如有。

PR 合并前：

- [ ] `bash scripts/check.sh all` 通过。
- [ ] GitHub Actions 全绿（至少 docs、links、secrets）。
- [ ] 文档、CHANGELOG、ADR 已按影响更新。
- [ ] Skill 变更已同步索引与 `agents/openai.yaml`。
- [ ] 没有提交密钥、个人环境路径或临时文件。

## 规则变更

修改 `AGENTS.md` 或门禁规则时：

1. 说明触发原因（代理犯了什么错、哪个检查缺失）。
2. 保持规则可执行/可检查，使用稳定编号。
3. 同步更新 `docs/` 与 `CHANGELOG.md`。
4. 有取舍时新增 ADR；不要删除旧 ADR。

## Skill 变更

新增或修改 `skills/` 下的技能时：

1. `name` 必须与目录名一致，使用小写 kebab-case。
2. 同步 `agents/openai.yaml`：用户调用技能设置 `allow_implicit_invocation: false`。
3. 在 `skills/README.md` 登记；行为变化更新 `docs/skills.md` 与 `CHANGELOG.md`。
4. 运行 `bash scripts/check.sh skills`，再跑全量 `bash scripts/check.sh all`。
5. 架构或调用策略变化时补 ADR；不要只改文字。

## 发布

本模板不自动发版。需要发布时：

1. 将 `CHANGELOG.md` 的 `Unreleased` 改为 `## [x.y.z] - YYYY-MM-DD`。
2. 提交 `chore(release): prepare vX.Y.Z`。
3. 打 tag `vX.Y.Z` 并推送。
4. 如需 GitHub Release，使用同一段变更说明。
