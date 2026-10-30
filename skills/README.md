# Agent Skills

本目录是可加载的工作流，不是第二份行为契约。规则仍以 [AGENTS.md](../AGENTS.md) 为准；技能只回答“这一步怎么做”。

## 技能集

| 技能 | 调用方式 | 解决的问题 | 产物 |
| --- | --- | --- | --- |
| [clarify](discovery/clarify/SKILL.md) | 用户 `/clarify` | 需求含糊、范围不清 | 决策简报 |
| [research](discovery/research/SKILL.md) | 模型自动 / 用户 | 需要一手来源与引用 | 带来源的研究笔记 |
| [spec](delivery/spec/SKILL.md) | 用户 `/spec` | 把已对齐的需求变成可验收的规格 | spec + 可选 Issue |
| [implement](delivery/implement/SKILL.md) | 用户 `/implement` | 按 charter 做最小改动并验证 | diff + 验证记录 |
| [review](delivery/review/SKILL.md) | 模型自动 / 用户 | 按标准与 spec 审查改动 | 分级发现清单 |
| [handoff](operations/handoff/SKILL.md) | 用户 `/handoff` | 新会话接续当前工作 | 临时目录中的交接文档 |
| [retro](operations/retro/SKILL.md) | 用户 `/retro` | 改进代理的运行环境 | 最多 5 条改进建议 |

## 推荐流程

```text
clarify → spec → implement → review
                      │
                      ├─ handoff（换会话）
                      └─ retro（改环境）

research 在任何阶段按需触发
```

不是每个改动都要走全流程。小修可以直接 `implement`；需求清楚时可以跳过 `clarify`。

## 调用方式

- **用户调用**：只有人输入 `/name` 才会运行；用于有副作用、需要人决策的动作。
- **模型调用**：人也可以输入 `/name`，代理在任务匹配时会自动使用；用于只读分析。

Claude Code 用 `disable-model-invocation: true` 表达前者；Codex 在 `agents/openai.yaml` 中用 `policy.allow_implicit_invocation: false`。两者必须一致。

## 安装

```bash
bash scripts/install-skills.sh --dry-run   # 预览
bash scripts/install-skills.sh             # 链接到 ~/.claude/skills 和 ~/.agents/skills
```

脚本使用符号链接，`git pull` 后无需重装。已存在的真实目录会被跳过，不会被删除。

## 新增技能

1. 在 `skills/<bucket>/<name>/` 下创建 `SKILL.md`，`name` 必须与目录名一致。
2. 添加 `agents/openai.yaml`，填写 `interface.display_name` 和 `short_description`。
3. 在本文件表格中登记。
4. 运行 `bash scripts/check.sh skills`。
5. 行为变化写入 `CHANGELOG.md`；架构变化补 ADR。

完整规范见 [docs/skills.md](../docs/skills.md)。

## 设计来源

结构参考了 [mattpocock/skills](https://github.com/mattpocock/skills) 的 Agent Skills 组织方式（按需加载、user/model 调用分离、索引与校验），技能内容按 AgentDev Charter 的规则与产物重新设计。格式遵循 [Agent Skills 规范](https://agentskills.io/specification)。
