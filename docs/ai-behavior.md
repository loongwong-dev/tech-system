# AI 行为与指令文件

本模板只维护一份行为契约：[AGENTS.md](../AGENTS.md)。其他工具的指令文件是**适配层**，不是第二份规范。

## 为什么以 AGENTS.md 为主

`AGENTS.md` 是跨工具的事实标准：Codex、GitHub Copilot、Cursor、Gemini CLI、Aider、goose、Zed、Warp 等均支持或可配置读取。它有几个适合本模板的特性：

- 纯 Markdown，无必需字段，不绑定供应商。
- 支持目录层级：代理读取离目标文件最近的 `AGENTS.md`，适合未来拆分子项目。
- 与 README 分工清晰：README 面向人，AGENTS.md 面向代理。
- 社区规模大，迁移成本低。

官方说明：<https://agents.md/>（访问日期：2026-10-30）。

## 工具映射

| 工具 | 默认读取 | 与本模板的衔接方式 | 注意 |
| --- | --- | --- | --- |
| Codex | `AGENTS.md` | 直接读取根文件 | 支持嵌套文件，最近者优先 |
| GitHub Copilot | `.github/copilot-instructions.md`、`AGENTS.md`、`CLAUDE.md`、`GEMINI.md` | cloud agent、code review、CLI 读取 `AGENTS.md`；Chat on github.com 只读 `.github/copilot-instructions.md` | 不同 Copilot 界面的读取范围不同，见官方支持矩阵 |
| Cursor | `.cursor/rules/*.mdc`、`AGENTS.md` | 直接读取 `AGENTS.md` | `.mdc` 支持 glob 与 `alwaysApply`，适合路径级规则 |
| Claude Code | `CLAUDE.md`、`AGENTS.md` | v2.1.277+ 默认在没有 `CLAUDE.md`/`CLAUDE.local.md` 时读取 `AGENTS.md`；存在 `CLAUDE.md` 时只读它 | 可用 `/config` 的 Project instructions 调整；旧版本用 `CLAUDE.md` 导入 |
| Gemini CLI | `GEMINI.md` | 在 `.gemini/settings.json` 设置 `context.fileName` 为 `AGENTS.md` | 或保留 `GEMINI.md` 作为一行导入 |
| 其他代理 | 各自约定 | 优先配置为读取 `AGENTS.md` | 不要复制规则正文 |

### 兼容适配示例

Claude Code 的旧版本只读 `CLAUDE.md`。如需兼容，创建一行文件：

```markdown
@AGENTS.md
```

Cursor 的路径级规则适合放“某目录特有”的约束，例如未来接入代码后：

```markdown
---
globs: src/api/**/*.ts
alwaysApply: false
---

- 复用 `src/api/errors.ts` 的错误结构。
- 新增端点必须补充契约测试。
```

路径级规则 **MUST NOT** 重复根 `AGENTS.md` 的通用规则；只写该路径额外需要的内容。

## 分层与冲突

推荐层级从高到低：

1. 用户当前对话指令。
2. 离目标文件最近的 `AGENTS.md`。
3. 根 `AGENTS.md`。
4. `docs/` 说明。
5. 工具默认行为。

发生冲突时，代理应指出冲突并请求确认，而不是静默选择。`AGENTS.md` 第 0 节已把这条写成规则。

## 评判一条规则是否合格

| 维度 | 合格 | 不合格 |
| --- | --- | --- |
| 可执行 | “运行 `bash scripts/check.sh all`” | “确保质量” |
| 可检查 | “提交标题 ≤ 100 字符” | “写清楚的提交信息” |
| 有边界 | “除非用户明确要求，不 commit” | “谨慎操作” |
| 有理由 | “因为 CI 会拒绝非 Conventional Commits” | 只有命令，没有原因 |
| 不重复 | 工具能强制的交给 CI | 在三个文件里各写一遍 |

## 审计清单

每季度或规则明显失效时检查：

- [ ] 每条规则仍有对应工具或审查动作。
- [ ] 没有与 CI 重复的大段描述。
- [ ] 工具映射表中的文件名仍与官方文档一致。
- [ ] `AGENTS.md` 仍控制在 200 行左右，未变成百科。
- [ ] 新增规则有编号，删除规则后没有悬空引用。
- [ ] 适配文件（`CLAUDE.md`、`.cursor/rules` 等）没有复制正文。

## 常见反模式

- **多份正文**：`AGENTS.md`、`CLAUDE.md`、`.cursor/rules` 各写一套，最终漂移。
- **口号化**：写“保持代码优雅”，代理无法判断是否完成。
- **过度约束**：把一次性任务偏好写进全局规则，增加每次会话的上下文成本。
- **绕过门禁**：教代理用 `--no-verify` 或删除测试来“通过”。
- **密钥进文档**：示例使用真实令牌，或把内部地址写进模板。

## 参考

- AGENTS.md 标准：<https://agents.md/>
- Codex 自定义指令：<https://developers.openai.com/codex/guides/agents-md>
- Cursor Rules：<https://cursor.com/docs/rules>
- GitHub Copilot 仓库指令：<https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions>
- Claude Code 记忆机制：<https://code.claude.com/docs/en/memory>
- Gemini CLI 上下文文件：<https://geminicli.com/docs/cli/gemini-md>

访问日期：2026-10-30。平台行为可能变化，升级前请复核。
