# ADR-0004：用 Agent Skills 承载工作流

- 状态：Accepted
- 日期：2026-10-30
- 决策者：模板维护者
- 相关：[AGENTS.md](../../AGENTS.md)、[技能规范](../skills.md)

## 背景

`AGENTS.md` 已经覆盖规则、优先级和停止条件，但“怎么澄清需求、怎么写 spec、怎么审查”属于多步流程。把这些流程全部写进 `AGENTS.md` 会带来两个问题：

- 每次会话都加载，挤占上下文，稀释真正重要的约束。
- 流程会持续增长，最终变成无法审查的长文。

Agent Skills 提供了按需加载的开放格式：启动时只加载 `name` 与 `description`，命中任务时才读取正文。Claude Code、Codex 等工具已支持，适合承载流程而不复制规则。

参考实现 [mattpocock/skills](https://github.com/mattpocock/skills) 验证了这种分层可行，但它的技能集面向通用软件工程。本模板只取结构设计，技能内容围绕自身规则重写。

## 决策

1. 新增 `skills/`，按 `discovery`、`delivery`、`operations` 三类组织。
2. 首批 7 个技能：`clarify`、`research`、`spec`、`implement`、`review`、`handoff`、`retro`。
3. `AGENTS.md` 仍是唯一契约；技能只写流程，冲突时契约优先。
4. 有副作用或改变范围的技能设为用户调用；只读分析设为模型调用。
5. 用 `scripts/install-skills.sh` 符号链接到 `~/.claude/skills` 与 `~/.agents/skills`。
6. 用 `scripts/check.sh skills` 校验 frontmatter、命名、索引和调用策略。
7. 暂不打包为 Claude Code 插件；等公开分发或团队安装需求出现再评估。

## 后果

### 正面

- 规则与流程分离，`AGENTS.md` 保持精简。
- 技能按需加载，日常会话成本低。
- 技能可独立修改、替换或删除，不影响契约。
- 安装方式简单，`git pull` 即更新。

### 负面 / 代价

- 多一层目录与索引，新增技能需要同步维护。
- 不同工具对调用控制字段支持不一致。
- 符号链接安装不支持 Windows 原生环境。

### 风险与缓解

- 风险：技能与 `AGENTS.md` 冲突。
- 缓解：技能正文只引用规则；审查时以契约为准；`check.sh` 校验结构，人工审查语义。
- 风险：技能索引漂移。
- 缓解：`check.sh skills` 强制每个技能在 `skills/README.md` 登记。
- 风险：技能过多导致选择困难。
- 缓解：首批只保留 7 个；新增技能必须有真实失败场景。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 全部写进 `AGENTS.md` | 单一文件 | 上下文膨胀，流程难维护 | 与按需加载目标冲突 |
| 直接安装第三方技能集 | 开箱即用 | 规则不匹配，难以审查 | 模板需要自己的契约 |
| 打包成 Claude Code 插件 | 安装体验好 | 额外清单、版本与发布流程 | 个人阶段过重 |
| 只做文档，不做技能 | 零维护 | 代理不会主动加载 | 无法形成可执行流程 |

## 验证方式

- 命令：`bash scripts/check.sh skills`。
- 安装预演：`bash scripts/install-skills.sh --dry-run`。
- 负向测试：破坏 frontmatter、目录名、索引或调用策略，确认校验失败。
- 复核：每次新增/删除技能时同步索引与文档。

## 参考

- Agent Skills 规范：<https://agentskills.io/specification>（访问日期：2026-10-30）
- Claude Code Skills：<https://code.claude.com/docs/en/skills>（访问日期：2026-10-30）
- Codex Skills：<https://developers.openai.com/codex/skills>（访问日期：2026-10-30）
- 设计参考：<https://github.com/mattpocock/skills>（访问日期：2026-10-30）
