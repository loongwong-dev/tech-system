# Issue 驱动的开发流程

任务从哪里来、现在做到哪、改动如何回溯。

- **类型**：通用规范
- **覆盖范围**：任务结构、Issue 字段、标签与状态机、AI 会话协议、追溯链、进度查看、自动化检查。
- **不覆盖范围**：提交信息格式见 [CONTRIBUTING.md](../../CONTRIBUTING.md)；代码组织见 [代码规范](../design/engineering/code-standards.md)。
- **状态**：已定，流程变更需同步 `AGENTS.md` 与 ADR。

## 1. 为什么用 Issue

AI 辅助开发的主要风险不是写错代码，而是**过程不可见**：任务只活在对话里，进度靠回忆，改动说不清对应哪条需求。

Issue 是唯一能让「任务、进度、追溯」三件事同时可查的载体：

| 问题 | 传统做法 | Issue 做法 |
| --- | --- | --- |
| 在做哪个任务 | 记在脑子里 | 唯一的 `status:in-progress` |
| 进度如何 | 问一句答一句 | 按里程碑统计 closed / total |
| 为什么改这行 | 翻聊天记录 | commit 带 `(#N)`，点进去看背景 |
| 需求是否满足 | 凭印象 | Issue 里的验收标准逐条对照 |

## 2. 三层结构

```text
Milestone（阶段）
  └─ Epic（交付单元）  一个可独立验收的能力，例如「批量生成」
       └─ Task（原子任务）一次会话可完成，必须有可执行验收标准
```

| 层级 | 判据 | 生命周期 |
| --- | --- | --- |
| Milestone | 里程碑，有明确退出条件 | 周级 |
| Epic | 完成后用户能感知到变化 | 3–7 天 |
| Task | 一次会话能做完，且能独立验证 | 半天到一天 |

**超过两天拆不动，说明还没想清楚**。宁可先开一个 `[Spike]` 任务去调研。

## 3. Issue 必备字段

缺任一项不得进入 `status:ready`：

| 字段 | 要求 |
| --- | --- |
| 目标 | 一句话，动词开头 |
| 验收标准 | **可执行**：命令、断言或可观察结果，不接受「功能正常」 |
| 范围 / 非范围 | 明确不做什么，防止顺手扩大 |
| 涉及模块 | 从 `area:*` 中选，用于估算影响面 |
| 依据文档 | 指向 `docs/` 中的具体章节 |
| 依赖 | 被哪个 Issue 阻塞 |
| 测试要求 | 哪些用例必须写，对应 [测试策略](../design/engineering/testing.md) 的优先级 |

模板见 `.github/ISSUE_TEMPLATE/task.yml` 与 `epic.yml`。

### 3.1 验收标准与完成定义

两者不是一回事，缺一不可：

| 概念 | 回答 | 归属 |
| --- | --- | --- |
| 验收标准 | 这个功能是否按需求工作 | 本 Issue 特有 |
| 完成定义 | 交付物是否达到可合并的状态 | 所有 Issue 共用 |

验收标准随任务变化；完成定义固定六条，写在模板里：

1. 验收标准逐条有证据。
2. 失败路径与边界有测试。
3. 受影响文档已同步。
4. 行为变化记入 CHANGELOG，取舍记入 ADR。
5. 无调试代码、临时代码或未引用文件。
6. 可回滚，回滚方式已说明。

### 3.2 依据文档而非另写规格

不新建只服务 Issue 的规格副本。仓库中已有的需求或设计文档就是规格，Issue 是它的任务切片：

```text
规格章节 ──▶ Epic
              ├─ Task
              └─ Task
```

因此每个 Issue 必须写出「依据文档」的具体章节，评审时也以该章节为准。文档变化时，相关 Issue 需同步检查。

## 4. 标签与状态机

```text
status:ready ──▶ status:in-progress ──▶ status:review ──▶ status:done
                        │
                        └──▶ status:blocked ──▶ status:ready
```

| 标签 | 含义 | 谁改 |
| --- | --- | --- |
| `status:ready` | 可开工，验收标准已写清 | 创建者 |
| `status:in-progress` | 正在做，**全局只允许一个** | AI 开工时 |
| `status:blocked` | 被外部条件卡住 | 任何人 |
| `status:review` | 已交付，等人确认 | AI 收尾时 |
| `status:done` | 验收通过 | 人工 |

其余标签：

| 前缀 | 取值 |
| --- | --- |
| `kind:` | `epic`、`task` |
| `area:` | 可选，取值与 `.github/ISSUE_TEMPLATE/task.yml` 的「涉及模块」保持一致 |

任务类型不单独打标签，写在标题前缀：`[Feature]`、`[Bug]`、`[Chore]`、`[Spike]`。

状态只允许这五个。增加状态前先问：它能否由这五个推导出来。

## 5. AI 会话协议

三个检查点，缺一不可。

### 开工

1. 查看是否存在 `status:in-progress` 的 Issue；有则继续它，没有则从 `status:ready` 取一个。
2. 把标签改为 `status:in-progress`。
3. 发一条**计划评论**：要改哪些文件、实现顺序、验证方式。
4. 建分支 `task/<N>-<短横线描述>`。

### 过程中

1. 小步提交，每条 commit 的 subject 末尾带 `(#N)`。
2. 发现范围外的问题：**开新 Issue，不做**。
3. 遇到需要决策的分叉：在 Issue 里记录选项与取舍，再继续。

### 收尾

1. 逐条核对完成定义（第 3.1 节）。
2. 运行 `bash scripts/check.sh all`。
3. 发一条**交付评论**：改动文件、验证命令与输出摘要、未完成项与原因。
4. 开 PR，正文包含 `Closes #N`，并填写验收证据与自检项。
5. 按 [代码评审](code-review.md) 接受评审；**合并只能由维护者执行**。
6. 标签改为 `status:review`；人工确认后改 `status:done`。

计划与交付两条评论是黑盒仅有的两个可审计快照。缺少交付评论的 Issue 不允许进入 `review`。

## 6. 追溯链

每一环都可从任意一端走到另一端：

```text
需求章节 ── Issue ── 分支 ── commit ── PR ── 代码 / 测试
```

| 环节 | 约定 |
| --- | --- |
| Issue → 文档 | 正文写 `依据：docs/<领域>/<文档>.md 第 4 节` |
| 分支 → Issue | `task/123-add-quota` |
| commit → Issue | subject 末尾 `(#123)` |
| PR → Issue | 正文 `Closes #123` |
| 代码 → Issue | 仅在逻辑不直观处写 `// 见 #123`，不做全员标注 |

`scripts/check-trace.sh` 校验 commit 是否带 Issue 引用，用法见第 9 节。

## 7. 进度查看

不维护手工进度表，全部从 Issue 派生：

```bash
gh issue list --label status:in-progress      # 现在在做什么
gh issue list --milestone M1 --state open     # 当前阶段剩余
gh issue list --label status:blocked          # 被卡住的任务
```

里程碑完成率 = 该 Milestone 下 `closed` 数 / 总数。需要看板时用 GitHub Projects，它对 label 的映射是自动的，不构成第二个数据源。

## 8. 边界规则

| 规则 | 说明 |
| --- | --- |
| 没有 Issue 不改代码 | 包括「一行小修」；先建 Issue 再改 |
| 一个 Issue 一次会话 | 做不完就拆，不延长 |
| 不做范围外的事 | 发现的问题开新 Issue，当下不改 |
| 不把 Issue 当日记 | 过程写评论，状态只写标签 |
| 交付必须附验证 | 贴命令与结果，不接受「已实现」 |

## 9. 自动化检查

| 检查 | 方式 |
| --- | --- |
| commit 是否引用 Issue | `BASE_SHA=<base> bash scripts/check-trace.sh` |
| PR 是否关联 Issue | CI 检查 PR 正文包含 `Closes #N` |
| PR 是否完成评审 | 维护者 Approve；AI 不得合并 |
| 是否只有一个进行中任务 | `gh issue list --label status:in-progress` 人工或定时检查 |
| 代码规范与文档 | `bash scripts/check.sh all` |

`check-trace.sh` 默认不纳入 `all`：仓库初始化阶段的提交没有 Issue 可引用。

## 10. 初始化标签

在仓库首次启用时执行一次：

```bash
gh label create "status:ready"       --color 0E8A16
gh label create "status:in-progress" --color FBCA04
gh label create "status:blocked"     --color D93F0B
gh label create "status:review"      --color 1D76DB
gh label create "status:done"        --color 5319E7
gh label create "kind:epic"          --color BFD4F2
gh label create "kind:task"          --color C5DEF5
# area:* 按项目模块补充，取值与 task.yml 的「涉及模块」一致
gh label create "area:api"           --color EDEDED
```

## 11. 反模式

| 反模式 | 后果 |
| --- | --- |
| 同时开三个 `in-progress` | 「在做什么」又有歧义，WIP=1 失效 |
| 验收标准写成「功能可用」 | 交付无法判定，只能凭感觉 |
| 一个 Issue 做一周 | 中间状态不可见，出问题无法回退 |
| 用标签表达十种状态 | 标签失去机器可读性 |
| 计划评论写完不更新 | 读者以为仍在按原计划进行 |
| 收尾不写验证输出 | 黑盒没有被打开，只是换了个位置 |
