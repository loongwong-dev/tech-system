---
name: spec
description: "Turn an agreed requirement into a verifiable spec with vertical slices, and publish it when the repo uses an issue tracker."
disable-model-invocation: true
argument-hint: "Optional issue, spec, or scope reference"
---

# Spec

把已经对齐的需求写成可执行、可验收的规格。**不要再访谈**；只综合当前对话、代码与文档里已有的事实。

## 先读

- `AGENTS.md` 第 1、3、5 节。
- 相关 `docs/`、现有 spec、ADR、issue。
- 当前实现与测试入口；确认仓库已有的检查命令。

使用项目已有术语；没有术语表时，用代码中的命名，不要另造一套。

## 写什么

```markdown
# <标题>

## 问题
从使用者视角描述现状与痛点。

## 结果
完成后能观察到什么变化。

## 不做什么
明确排除项，防止范围蔓延。

## 验收条件
- [ ] 可判定条件；尽量附命令或场景。

## 切片
按可独立验证的垂直切片排列，每片包含：
1. 改动范围
2. 验收方式
3. 依赖的前置切片

## 风险与回滚
最可能失败的环节；如何撤回。

## 验证
需要运行的命令、需要人工观察的界面或行为。

## 文档影响
需要更新的 README/docs/ADR/CHANGELOG；没有则写“无”。

## 待确认
仍然阻塞实现的问题；没有则写“无”。
```

## 切片规则

- 每个切片必须能单独验证，避免“先写完全部测试再写实现”。
- 优先改现有接缝，不为新需求凭空造抽象。
- 一个切片只产生一个可审查的改动。
- 切片之间有依赖时写清楚顺序。

## 发布

按仓库现状选择一种，不要重复创建：

1. 仓库使用 issue tracker：创建 issue，标题用 spec 标题，正文用 spec 内容；已有 issue 则更新。
2. 仓库有 `specs/`：写入 `specs/NNNN-<slug>.md`。
3. 都没有：写入 `specs/0001-<slug>.md`，并在回复中说明新建了目录。

如果写入 `docs/` 下，必须同步 `docs/README.md` 索引，否则 `check.sh docs` 会失败。

## 结束

- 每个验收条件都有验证方式。
- 没有未说明的阻塞项。
- 不实现、不提交；建议下一步运行 `/implement`。
