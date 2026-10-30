# AgentDev Charter

**面向 AI 辅助开发的、语言与框架无关的项目模板。** 它不提供业务代码，而是约束 AI 代理在仓库中的工作方式：读什么、怎么改、如何写文档、如何提交、如何通过门禁、遇到冲突怎么办。

仓库名：`agent-dev-charter`。

## 定位

- **单一行为契约**：根级 [AGENTS.md](AGENTS.md) 是唯一权威规则；其他 AI 工具文件只做适配。
- **可执行而非口号**：每条规则尽量对应命令、检查项或明确的审查动作。
- **GitHub 门禁**：GitHub Actions 做完整检查。
- **docs-as-code**：文档与代码同仓、同审查、同 CI。
- **个人可维护**：默认零运行时依赖，不引入包管理器、静态站点生成器或复杂流水线。

### 不是什么

- 不是业务项目脚手架，不生成语言/框架代码。
- 不是 AI 代理实现，不包含提示词工程平台。
- 不是“大而全”的工程规范；只保留能被执行或审查的规则。

## 设计原则

| 原则 | 落地方式 |
| --- | --- |
| 单一来源 | `AGENTS.md` 管行为，`README.md` 管定位，`docs/` 管解释，ADR 管取舍 |
| 最小依赖 | 核心检查只用 bash、git 与 POSIX 文本工具 |
| 平台中立 | 不绑定语言、框架、运行时；平台差异放在适配层 |
| 可检查 | `scripts/check.sh` + CI；规则必须能失败 |
| 可演进 | 规则编号、ADR、CHANGELOG、季度审计 |
| 不过度设计 | 可选工具只写文档，不默认落文件 |

## 快速开始

### 1. 复制模板

```bash
git clone <your-repo-url> agent-dev-charter
cd agent-dev-charter
```

或直接在 GitHub 使用“Use this template / 使用模板创建仓库”。

### 2. 确认模板信息

- `CHANGELOG.md`：保留或清空 `Unreleased`，按需把 `0.1.0` 改为你的起始版本。
- `SECURITY.md`：确认平台私密报告渠道可用；如需邮箱或 IM，再补充。
- 如需固定维护者，再添加 `.github/CODEOWNERS`。
- 许可证已附带 Apache-2.0；如需更换，替换 `LICENSE` 并同步本段。

### 3. 运行本地检查

```bash
bash scripts/check.sh all
```

### 4. 启用 GitHub 门禁

- 推送到 `main` 或发起 PR，`.github/workflows/quality.yml` 自动运行。

## 目录结构

```text
.
├── AGENTS.md                     # AI 行为契约（唯一权威）
├── LICENSE                       # Apache License 2.0
├── README.md                     # 本文件：定位与用法
├── CONTRIBUTING.md               # 人的协作流程
├── CHANGELOG.md                  # 使用者可感知的变更
├── SECURITY.md                   # 安全与泄露处理
├── .editorconfig                 # 跨编辑器格式基线
├── .gitattributes                # 换行与二进制标记
├── .gitignore                    # 本地环境、产物、密钥
├── .markdownlint-cli2.jsonc      # Markdown 规则
├── .lychee.toml                  # 链接检查配置
├── .github/
│   ├── workflows/quality.yml     # GitHub Actions
│   ├── ISSUE_TEMPLATE/           # Issue 表单
│   ├── pull_request_template.md  # GitHub PR 模板
│   └── dependabot.yml            # Actions 月度更新
├── docs/
│   ├── README.md                 # 文档索引与入口
│   ├── governance.md             # 文档治理与组织
│   ├── ai-behavior.md            # AI 指令生态与映射
│   ├── ci-cd.md                  # 门禁矩阵与排障
│   ├── configuration.md          # 配置职责与可选性
│   └── adr/                      # 决策记录
└── scripts/
    └── check.sh                  # 语言无关检查入口
```

## 质量门禁

| 检查 | 本地 | GitHub |
| --- | --- | --- |
| 文档结构、H1、索引、ADR 命名 | ✅ | ✅ |
| 站内链接 | ✅ | ✅ |
| 外链 | 可选（lychee） | ✅ |
| Markdown 完整规则 | 可选（markdownlint） | ✅ |
| 提交信息 | ✅ | ✅ |
| 密钥扫描 | 基线 | gitleaks |
| 文本格式 | ✅ | ✅ |

详细说明与失败处理见 [docs/ci-cd.md](docs/ci-cd.md)。

### 常用命令

```bash
bash scripts/check.sh all        # 全量
bash scripts/check.sh docs       # 文档结构 + 站内链接
bash scripts/check.sh format     # 换行/空白/Tab
bash scripts/check.sh commits    # Conventional Commits
bash scripts/check.sh secrets    # 密钥基线
CHECK_EXTERNAL_LINKS=1 bash scripts/check.sh links
```

## 如何使用这套规范

### 给 AI 代理

1. 把仓库根目录作为工作目录启动代理。
2. 确认代理读取 `AGENTS.md`；不支持的工具按 [docs/ai-behavior.md](docs/ai-behavior.md) 配置。
3. 任务描述中给出目标文件、验收标准和禁止事项。
4. 要求代理在结束前运行 `bash scripts/check.sh all` 并报告结果。

### 给自己

1. 改规则前先问：能否被检查？不能就写成明确的审查动作。
2. 新规则先加编号，再加检查，最后更新文档与 CHANGELOG。
3. 有取舍就写 ADR，不要只留在对话里。
4. 每季度删一次过期规则；模板越大，代理越容易忽略重点。

### 接入实际项目

- 保留 `AGENTS.md`、`docs/`、`scripts/check.sh` 和 CI。
- 把语言特定的 lint/test 加到 `scripts/check.sh` 的子命令或 CI job，不要塞进核心脚本。
- 子项目需要不同规则时，在子目录放嵌套 `AGENTS.md`，最近者优先。
- 需要路径级 AI 规则时，再加 `.cursor/rules/*.mdc`，只写局部差异。

## 维护

| 频率 | 动作 |
| --- | --- |
| 每次变更 | `bash scripts/check.sh all`；更新 CHANGELOG |
| 每月 | 合并 Dependabot 的 Actions 更新；抽查外链 |
| 每季度 | 审计 `AGENTS.md` 规则、工具映射、排除项 |
| GitHub Actions 变更时 | 复核 workflow 语法与权限 |

维护细节见 [docs/configuration.md](docs/configuration.md)。

## 已知限制

- `check.sh` 不解析 Markdown AST，不检查页内锚点。
- `CHECK_EXTERNAL_LINKS=1` 需要本机安装 `lychee`；缺失时检查失败。
- gitleaks 在 push/PR 只扫描事件范围，PR 只取 API 返回的一页提交；完整历史由每日定时任务覆盖。
- 密钥扫描是轻量基线，不能替代 gitleaks。
- 模板默认不含 `CODEOWNERS`、`pre-commit`，避免替使用者做工具选择。

## 文档

- [文档索引](docs/README.md)
- [文档治理与组织](docs/governance.md)
- [AI 行为与指令文件](docs/ai-behavior.md)
- [CI/CD 与质量门禁](docs/ci-cd.md)
- [项目配置说明](docs/configuration.md)
- [ADR 索引](docs/adr/README.md)

## 设计依据

本模板的设计基于 2026-10-30 的联网调研，主要来源：

- [AGENTS.md 标准](https://agents.md/)
- [OpenAI Codex：AGENTS.md](https://developers.openai.com/codex/guides/agents-md)
- [Cursor Rules](https://cursor.com/docs/rules)
- [GitHub Copilot 仓库指令](https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions)
- [Claude Code 记忆机制](https://code.claude.com/docs/en/memory)
- [Write the Docs：Docs as Code](https://www.writethedocs.org/guide/docs-as-code/)
- [ADR](https://adr.github.io/)
- [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)
- [Conventional Commits](https://www.conventionalcommits.org/zh-hans/v1.0.0/)

长期取舍按 [ADR 流程](docs/adr/README.md) 记录。

## 许可证

本项目采用 [Apache License 2.0](LICENSE)。使用时保留 `LICENSE` 与必要的版权声明；修改后的分发需遵守许可证第 4 条的标注要求。
