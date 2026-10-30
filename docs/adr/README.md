# ADR 索引与流程

ADR（Architectural Decision Record）记录**有长期影响的选择**：为什么这样定、放弃了什么、以后如何复核。它不是任务日志，也不是教程。

## 何时写 ADR

满足任一条件就写：

- 选择或替换工具、平台、目录结构、规则体系。
- 引入新的强制门禁或依赖。
- 做出以后很难回滚的决定。
- 对同一问题存在两个以上合理方案，需要留下取舍。

纯文案修正、链接更新、模板错别字不需要 ADR。

## 文件约定

```text
docs/adr/
├── README.md
├── template.md
└── NNNN-lowercase-kebab.md
```

- 模板仓库不预置 ADR；派生项目从 `0001` 开始记录自己的决策。
- 编号四位递增，不复用、不跳号太久。
- 文件名：`NNNN-lowercase-kebab.md`。
- 状态：`Proposed`、`Accepted`、`Superseded by NNNN`、`Deprecated`。
- 新 ADR **MUST** 登记到本文件与 `docs/README.md`。

## 流程

1. 复制 `template.md` 为 `NNNN-title.md`。
2. 填写上下文、决策、后果、备选方案。
3. 在 PR 中说明；涉及规则变更时同步 `AGENTS.md` 与 `CHANGELOG.md`。
4. 合并后状态改为 `Accepted`。
5. 决策被推翻时，不删除旧 ADR；新增一条并标注 `Superseded by`。

## 索引

- [ADR 模板](template.md)

## 参考

- ADR 官方站点：<https://adr.github.io/>
- Michael Nygard 的原始文章：<https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions>
- MADR 模板：<https://adr.github.io/madr/>

访问日期：2026-10-30。
