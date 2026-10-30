---
name: review
description: "Review a diff, branch, PR, or completed change against AGENTS.md, documented repo standards, and the originating spec. Use when the user asks for a review, verification, or a merge decision."
---

# Review

审查一个固定点以来的改动，分别回答两个问题：

- **标准**：是否遵守 `AGENTS.md`、`CONTRIBUTING.md`、`docs/` 与仓库工具链？
- **规格**：是否忠实实现了原始 issue/spec？

两个轴分开报告，不要合并或重新排序；一个轴通过不代表另一个轴通过。

## 1. 固定比较范围

- 用户指定了 commit、分支、tag 或 PR：以它为 fixed point。
- 未指定：用 `git merge-base HEAD main`（没有 `main` 则 `master`）。
- 有未提交改动时，单独说明并纳入审查。
- 先确认引用可解析、diff 非空；失败就停在这里。

收集：

```bash
git diff <fixed-point>...HEAD
git log <fixed-point>..HEAD --oneline
git diff --stat <fixed-point>...HEAD
```

## 2. 找规格来源

按顺序查找：

1. commit message 中的 `#123`、`Closes #45` 等引用。
2. 用户传入的 spec/issue 路径或编号。
3. `specs/`、`docs/` 中与分支名匹配的文件。
4. 都没有：说明“无 spec”，只做标准轴。

## 3. 标准轴

读取并引用：

- `AGENTS.md`：A（上下文）、C（改动约束）、D（文档）、G（提交/PR）、V（验证）、S（安全）。
- `CONTRIBUTING.md` 与相关 `docs/`。
- `scripts/check.sh` 与 CI 配置，确认哪些已由工具强制，避免重复报风格问题。

重点检查：

- 是否最小改动、是否夹带无关重构。
- 是否删除/弱化无关测试或检查。
- 是否绕过门禁（`|| true`、`--no-verify`、放宽规则）。
- 文档、CHANGELOG、ADR 是否按影响更新。
- 提交信息是否符合 Conventional Commits。
- 是否引入未说明的依赖、密钥或权限。
- 是否真的运行了验证，而不是只声称“已测试”。

## 4. 规格轴

逐条对照 spec：

- 哪些要求缺失或只完成一部分。
- 哪些行为是 spec 没要求的范围蔓延。
- 哪些要求看似实现，但行为可能不对。

每条引用 spec 原文。没有 spec 时跳过此轴，并明确说明。

## 5. 运行验证

- 优先 `bash scripts/check.sh all`。
- 否则运行仓库文档中的检查命令。
- 记录命令、退出码与关键输出；不要只写“通过”。
- 无法运行时说明原因和剩余风险。

## 6. 输出

```markdown
## 结论
可合并 / 需修改 / 阻塞（一句话理由）

## 标准
- [阻塞] 文件:行 — 问题；引用规则；修复建议

## 规格
- [缺失] 要求原文 — 现状；建议

## 验证
- `命令` → 结果

## 优先修复
1. ...
```

严重级别只用：`阻塞`、`重要`、`次要`、`建议`。不要报工具已经强制的格式问题；不要为了凑数列无关建议。

## 边界

- 只审查，不修改代码。
- 不替用户做合并决定；给出结论与依据。
- 发现安全问题（密钥、权限、注入）一律标为阻塞。
