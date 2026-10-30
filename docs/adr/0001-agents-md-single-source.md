# ADR-0001：以 AGENTS.md 作为单一行为契约

- 状态：Accepted
- 日期：2026-10-30
- 决策者：模板维护者
- 相关：[AGENTS.md](../../AGENTS.md)、[AI 行为说明](../ai-behavior.md)

## 背景

AI 编码工具各自使用不同的指令文件：Codex 读 `AGENTS.md`，Claude Code 读 `CLAUDE.md`，Cursor 读 `.cursor/rules/*.mdc`，Copilot 读 `.github/copilot-instructions.md`，Gemini CLI 读 `GEMINI.md`。

如果每个文件都维护一份正文，规则会漂移：改了一处，忘了另一处，代理在不同工具里表现不一致。个人模板没有精力做持续同步。

## 决策

以根级 `AGENTS.md` 作为唯一行为契约。其他工具通过以下方式适配：

- 原生支持 `AGENTS.md` 的工具直接读取。
- Claude Code 新版本直接读取；旧版本用 `CLAUDE.md` 一行 `@AGENTS.md` 导入。
- Gemini CLI 通过 `.gemini/settings.json` 将 `context.fileName` 指向 `AGENTS.md`。
- Cursor 需要路径级规则时，才新增 `.cursor/rules/*.mdc`，且不复制通用规则。

## 后果

### 正面

- 规则只有一个权威来源，审查和修改成本低。
- 换工具时不需要重写规范。
- 未来拆分子项目时，可用嵌套 `AGENTS.md` 覆盖局部规则。

### 负面 / 代价

- 少数旧版工具需要适配文件或配置。
- `AGENTS.md` 过长会稀释注意力，需要控制在 200 行左右。
- 不同工具对冲突的解析不完全一致，仍需人工判断。

### 风险与缓解

- 风险：适配文件逐渐复制正文，重新出现漂移。
- 缓解：`docs/ai-behavior.md` 明确“适配层只允许引用”；季度审计检查。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 以 `CLAUDE.md` 为主 | Claude Code 原生 | 其他工具需适配，偏供应商 | 与语言/工具无关目标冲突 |
| 每个工具一份正文 | 各工具体验最佳 | 必然漂移，维护成本高 | 个人模板不可持续 |
| 只用 `.cursor/rules` | 支持 glob 与自动附加 | 绑定 Cursor，纯 Markdown 场景过重 | 不满足跨工具目标 |

## 验证方式

- 检查：`AGENTS.md` 存在且只有一个 H1；`docs/ai-behavior.md` 映射表完整。
- 命令：`bash scripts/check.sh docs`。
- 复核：每季度检查工具官方文档中的文件名是否变化。

## 参考

- AGENTS.md：<https://agents.md/>（访问日期：2026-10-30）
- OpenAI Codex 指南：<https://developers.openai.com/codex/guides/agents-md>（访问日期：2026-10-30）
- Claude Code 记忆机制：<https://code.claude.com/docs/en/memory>（访问日期：2026-10-30）
- Cursor Rules：<https://cursor.com/docs/rules>（访问日期：2026-10-30）
- GitHub Copilot 仓库指令：<https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions>（访问日期：2026-10-30）
