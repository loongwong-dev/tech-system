# Agent Skills 工作流

`AGENTS.md` 约束“必须/禁止做什么”，技能约束“某类任务按什么步骤做”。两者分工明确：

| 层 | 位置 | 加载方式 | 内容 |
| --- | --- | --- | --- |
| 契约 | `AGENTS.md` | 每次会话 | 规则、优先级、停止条件 |
| 流程 | `skills/**/SKILL.md` | 命中时才加载 | 步骤、模板、检查项 |
| 门禁 | `scripts/check.sh`、CI | 每次检查 | 可自动判定的规则 |

技能**不得**覆盖或放宽 `AGENTS.md`。冲突时以契约为准，并修正技能。

## 目录结构

```text
skills/
├── README.md
├── discovery/          # 先弄清问题
│   ├── clarify/
│   └── research/
├── delivery/           # 产出与验证
│   ├── spec/
│   ├── implement/
│   └── review/
└── operations/         # 会话与环境维护
    ├── handoff/
    └── retro/
```

每个技能目录至少包含：

```text
<name>/
├── SKILL.md            # 必需：frontmatter + 步骤
└── agents/
    └── openai.yaml     # 可选：Codex 展示与调用策略
```

## 调用模式

| 模式 | 判定 | Claude Code | Codex |
| --- | --- | --- | --- |
| 用户调用 | 有副作用、需要人决策、会改变范围 | `disable-model-invocation: true` | `policy.allow_implicit_invocation: false` |
| 模型调用 | 只读分析，代理可自行判断相关性 | 省略该字段 | 省略 policy |

当前分配：

- 用户调用：`clarify`、`spec`、`implement`、`handoff`、`retro`
- 模型调用：`review`、`research`

## 编写规则

1. **一个技能只做一件事**。步骤超过 100 行时，拆出 `references/`，不要继续膨胀 `SKILL.md`。
2. **描述决定触发**。模型调用技能的 `description` 要写清“做什么 + 何时用”；用户调用技能的描述给人看，不堆触发词。
3. **引用而非复制**。需要规则时指向 `AGENTS.md` 的具体章节，不复制整段。
4. **语言无关**。命令从仓库发现（`scripts/check.sh`、`Makefile`、`package.json` 等），不假设技术栈。
5. **可验证结束**。每个技能都要有“完成条件”，最好能落到命令或检查结果。
6. **不自动提交**。除用户明确要求，技能不得 `git commit` / `push`。

## 校验

```bash
bash scripts/check.sh skills
```

检查项：

- 每个 `SKILL.md` 有 `name`、`description` frontmatter。
- `name` 与目录名一致，且符合 `[a-z0-9-]`。
- `description` 非空且不超过 1024 字符。
- 用户调用技能在 `agents/openai.yaml` 中关闭隐式调用。
- 每个技能都在 `skills/README.md` 登记。
- 技能名不重复，`SKILL.md` 不超过 500 行。

官方提供 `skills-ref` 校验器（需单独安装）：

```bash
skills-ref validate ./skills/delivery/spec
```

## 安装与兼容

```bash
bash scripts/install-skills.sh
```

安装位置：

- `~/.claude/skills`：Claude Code 个人技能。
- `~/.agents/skills`：Codex 及其他兼容 Agent Skills 的工具。

其他工具的支持情况不同：

- Cursor、Copilot 等若不读取 Agent Skills，可把 `SKILL.md` 内容作为规则或提示词引用；不要复制成第二份正文。
- `disable-model-invocation` 与 `agents/openai.yaml` 是工具扩展，不属于 Agent Skills 核心规范；其他工具可忽略。

## 为什么不打包成插件

当前是个人模板，优先可读、可改、零构建：

- 符号链接安装足够，`git pull` 即更新。
- 插件需要额外清单、版本同步和发布流程。
- 等公开分发或需要团队统一安装时，再评估 `.claude-plugin/` 打包。

该取舍记录在 [ADR-0004](adr/0004-agent-skills-workflow.md)。

## 参考

- Agent Skills 规范：<https://agentskills.io/specification>
- Claude Code Skills：<https://code.claude.com/docs/en/skills>
- Codex Skills：<https://developers.openai.com/codex/skills>
- 设计参考：<https://github.com/mattpocock/skills>

访问日期：2026-10-30。技能格式遵循开放规范；工具扩展字段可能随版本变化。
