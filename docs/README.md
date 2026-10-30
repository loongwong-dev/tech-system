# 文档索引与规范

`docs/` 是本模板的**规范来源**。根目录文件负责入口和契约，`docs/` 负责解释“为什么”和“怎么维护”。

## 目录约定

```text
docs/
├── README.md          # 本文件：索引与写作规则
├── ai-behavior.md     # AI 指令文件生态、映射与审计
├── ci-cd.md           # 双平台门禁、运行方式与排障
├── configuration.md   # 各配置文件职责、可选/必需、维护方式
└── adr/               # 架构与规范决策记录
    ├── README.md
    ├── template.md
    └── NNNN-*.md
```

- 文件名使用小写 kebab-case，例如 `ci-cd.md`。
- 每个文档只有一个一级标题；二级标题用于主题，三级标题用于细节。
- 新增文档 **MUST** 在本文件登记，否则 `scripts/check.sh docs` 会失败。
- ADR 使用四位编号，例如 `0004-add-link-checker.md`，不复用编号。
- 站内链接使用相对路径；外链使用完整 URL。
- 文档默认中文；命令、路径、协议名、代码标识符保留英文。

## 文档类型

| 类型 | 位置 | 回答的问题 | 更新时机 |
| --- | --- | --- | --- |
| 入口 | `README.md` | 这是什么、怎么用 | 定位、用法、结构变化 |
| 契约 | `AGENTS.md` | AI 必须/禁止做什么 | 行为规则变化 |
| 贡献指南 | `CONTRIBUTING.md` | 人如何提交变更 | 流程变化 |
| 说明 | `docs/*.md` | 为什么这样设计、如何维护 | 机制或配置变化 |
| 决策 | `docs/adr/NNNN-*.md` | 当时选了什么、放弃了什么 | 不可逆或有争议的选择 |
| 变更 | `CHANGELOG.md` | 使用者能感知到什么变化 | 每次合并 |

## 写作规则

1. **可执行**：优先写命令、路径、检查项，而不是“注意规范”。
2. **单一来源**：同一事实只在一处详细说明，其他位置链接过去。
3. **可验证**：涉及行为的描述应能通过 `bash scripts/check.sh all` 或 CI 验证。
4. **可维护**：不复制大段上游文档；引用官方链接并注明访问日期。
5. **不堆砌**：能删的段落就删；模板不是论文。
6. **索引格式**：本文件中的索引链接写成 `(path)`，不要带 `./` 前缀；否则 `check.sh docs` 会判定未登记。

## 索引

### 核心说明

- [AI 行为与指令文件](ai-behavior.md)：`AGENTS.md` 的定位、工具映射、冲突处理。
- [CI/CD 与质量门禁](ci-cd.md)：GitHub Actions、Gitee Go、本地检查与排障。
- [项目配置说明](configuration.md)：每个配置文件的职责、可选性与维护方式。
- [Agent Skills 工作流](skills.md)：技能格式、调用方式、安装与校验。

### 决策记录

- [ADR 索引与流程](adr/README.md)
- [ADR 模板](adr/template.md)
- [ADR-0001：以 AGENTS.md 作为单一行为契约](adr/0001-agents-md-single-source.md)
- [ADR-0002：语言无关的 Shell 门禁](adr/0002-language-agnostic-shell-gates.md)
- [ADR-0003：GitHub 为主、Gitee 对等的双平台 CI](adr/0003-dual-platform-ci.md)
- [ADR-0004：用 Agent Skills 承载工作流](adr/0004-agent-skills-workflow.md)
