---
name: research
description: "Investigate a question against primary sources and write a cited Markdown note. Use when facts, official docs, API behavior, or version claims need verification."
---

# Research

回答一个具体问题，并留下可复核的来源。不要用二手总结替代一手资料。

## 1. 定义问题

先写清：

- 要回答的问题（一句话）。
- 这个答案会影响哪个决定或改动。
- 什么算“足够可信”：官方文档、源码、规范、第一方 API，还是实测。

问题太大时先拆小；不要一边研究一边改代码。

## 2. 查来源

优先级：

1. 官方文档、规范、源码、第一方 API。
2. 项目自身的配置、CI 日志、版本记录。
3. 二手文章只用于找线索，不作为最终依据。

每条结论记录：

- 来源 URL 或文件路径。
- 访问日期。
- 原文关键句（短引用）。
- 你的推断，以及推断与原文的边界。

来源冲突时并列呈现，不要静默选一个。

## 3. 写笔记

优先跟随仓库既有约定。没有约定时：

- 写入 `docs/research/YYYY-MM-DD-<slug>.md`。
- 如果放在 `docs/` 下，必须同步 `docs/README.md` 索引，否则 `check.sh docs` 会失败。
- 不想进入文档树时，写到根目录 `research/`，并在回复中说明。

结构：

```markdown
# <问题>

## 结论
3 句以内。

## 证据
- 结论 — 来源 — 原文/观察

## 不确定性
- 未验证、版本差异、环境限制。

## 来源
- 标题：<URL>（访问日期：YYYY-MM-DD）
```

## 4. 结束

- 在回复中给出笔记路径与 3 条以内摘要。
- 明确标注“未验证”的部分。
- 不做实现；把结论交给 `/spec` 或 `/implement`。
