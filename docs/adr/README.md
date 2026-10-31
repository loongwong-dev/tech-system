# ADR 索引与流程

ADR（Architectural Decision Record）记录**有长期影响的选择**：为什么这样定、放弃了什么、以后如何复核。它不是任务日志，也不是教程。

## 何时写 ADR

满足任一条件就写：

- 选择或替换工具、平台、目录结构、规则体系。
- 引入新的强制门禁或依赖。
- 做出以后很难回滚的决定。
- 对同一问题存在两个以上合理方案，需要留下取舍。

纯文案修正、链接更新、模板错别字不需要 ADR。

新增前 **MUST** 先查本文的「主题索引」与现有 ADR：同一主题的演进优先修订，不另开新文件。

## 单一主题

- **一个 ADR 只回答一个决策**。标题里出现「和」「以及」通常意味着应拆成多条。
- **一个 ADR 只服务一个主题或一个模块**。不把多个模块的决策写进同一文件。
- **同一主题只保留一条现行 ADR**；历史条目在主题索引中列出，只读。
- 同一决策下的子规则可以放在一起；独立决策必须拆开。
- 修订只追加与本题相关的内容，不借机塞入无关模块或新主题。

## 修订与新增

| 情况 | 动作 |
| --- | --- |
| 同一主题的参数、路径、范围微调，结论不变 | 原地修订；状态保持 `Accepted`；追加「修订记录」 |
| 同一主题新增兼容约束 | 原地修订；追加「修订记录」 |
| 决策被推翻或方向改变 | 新增 ADR；旧文状态改为 `Superseded by NNNN`；不删除旧文 |
| 新模块或新主题 | 新增 ADR；不复用无关 ADR |
| 一个 ADR 已混入多个独立决策 | 拆成多条 ADR；原文保留或标 `Superseded by`，不在原文继续堆叠 |

修订规则：

1. 保留原背景、原决策与原后果，不重写历史。
2. 在文末追加「修订记录」，写明日期、改了什么、为什么、影响哪些文档。
3. 决策方向变化时不得只改正文，必须新增 ADR 并在旧文标注 `Superseded by`。
4. 修订不改变 ADR 编号与文件名；编号只增不改，不新增索引条目，只更新主题索引。
5. 修订后同步「主题索引」、相关链接与 `CHANGELOG.md`。

「修订记录」格式：

```markdown
## 修订记录

- 2026-10-31：允许同主题原地修订；原因是从 0006 到 0014 的连续新增造成同一主题膨胀。
```

## 文件约定

```text
docs/adr/
├── README.md
├── template.md
└── NNNN-lowercase-kebab.md
```

- 新项目从 `0001` 开始记录自己的决策；本仓库已记录本项目决策。
- 编号四位递增，不复用、不跳号太久。
- 文件名：`NNNN-lowercase-kebab.md`。
- 状态：`Proposed`、`Accepted`、`Superseded by NNNN`、`Deprecated`。
- 新 ADR **MUST** 登记到本文件；ADR 目录由 `docs/README.md` 统一索引。

## 流程

1. 查「主题索引」与现有 ADR，判断是修订还是新增。
2. 修订：在原 ADR 追加「修订记录」；新增：复制 `template.md` 为 `NNNN-title.md`。
3. 填写上下文、决策、后果、备选方案；只覆盖一个决策。
4. 在 PR 中说明；涉及规则变更时同步 `AGENTS.md` 与 `CHANGELOG.md`。
5. 合并后状态改为 `Accepted`。
6. 决策被推翻时，不删除旧 ADR；新增一条并标注 `Superseded by`。

## 主题索引

修订时以「现行」为准；「历史」只读，不再追加内容。

| 主题 | 现行 | 历史 |
| --- | --- | --- |
| 后端技术栈 | [ADR-0001](0001-rust-backend.md) | — |
| 后台任务队列 | [ADR-0002](0002-postgres-job-queue.md) | — |
| 图片渲染 | [ADR-0003](0003-typst-renderer.md) | — |
| 多租户隔离 | [ADR-0004](0004-tenant-isolation.md) | — |
| 前端拆分与契约 | [ADR-0005](0005-two-frontends.md) | — |
| 响应式 Web | [ADR-0007](0007-responsive-web-client.md) | — |
| 代码规范 | [ADR-0008](0008-code-standards.md) | — |
| Issue 流程 | [ADR-0009](0009-issue-driven-development.md) | — |
| 合并与人工评审 | [ADR-0010](0010-mandatory-review.md) | — |
| 评审标准 | [ADR-0011](0011-review-standard-separation.md) | — |
| 文档组织 | [ADR-0014](0014-module-docs-layout.md) | [ADR-0006](0006-docs-structure.md)、[ADR-0012](0012-process-design-split.md)、[ADR-0013](0013-design-engineering-standards.md) |
| ADR 生命周期 | [ADR-0015](0015-adr-revision-policy.md) | — |

## 索引

- [ADR 模板](template.md)
- [ADR-0001：后端采用 Rust + axum + sqlx](0001-rust-backend.md)
- [ADR-0002：后台任务使用 PostgreSQL 自建队列](0002-postgres-job-queue.md)
- [ADR-0003：反馈图片默认使用 typst 渲染](0003-typst-renderer.md)
- [ADR-0004：多租户隔离采用应用层约束 + PostgreSQL RLS 双连接池](0004-tenant-isolation.md)
- [ADR-0005：C 端与管理端拆为两个前端应用，契约由后端生成](0005-two-frontends.md)
- [ADR-0006：文档按主题分层并支持目录级索引](0006-docs-structure.md)（Superseded by 0014）
- [ADR-0007：C 端采用响应式 Web，暂不做小程序](0007-responsive-web-client.md)
- [ADR-0008：代码规范以可检查规则落地](0008-code-standards.md)
- [ADR-0009：以 Issue 作为开发任务的唯一来源](0009-issue-driven-development.md)
- [ADR-0010：合并前必须人工评审，AI 不得自行合并](0010-mandatory-review.md)
- [ADR-0011：评审标准与项目规则解耦，专项检查归入领域文档](0011-review-standard-separation.md)
- [ADR-0012：文档按性质拆分通用流程与本项目设计](0012-process-design-split.md)（Superseded by 0014）
- [ADR-0013：项目特有的代码与测试规范归入设计文档](0013-design-engineering-standards.md)（Superseded by 0014）
- [ADR-0014：文档按性质与功能模块组织](0014-module-docs-layout.md)
- [ADR-0015：ADR 允许同主题修订并保持单一主题](0015-adr-revision-policy.md)

## 参考

- ADR 官方站点：<https://adr.github.io/>
- Michael Nygard 的原始文章：<https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions>
- MADR 模板：<https://adr.github.io/madr/>

访问日期：2026-10-30。
