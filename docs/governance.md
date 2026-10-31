# 文档治理与组织

本文定义 `docs/` 的治理规则与组织方式，补充 [AGENTS.md](../AGENTS.md) 的 D1–D7 与 [CONTRIBUTING.md](../CONTRIBUTING.md)。两者冲突时以 `AGENTS.md` 为准。

- **类型**：模板治理

参考的社区做法：

- **Docs as Code**：文档与代码使用同一套版本控制、Issue、评审与自动化检查。
- **Diátaxis**：按 tutorial、how-to、reference、explanation 四类组织内容。
- **GitLab**：文档是产品信息的单一来源，采用 docs-first 与 topic types。
- **Google Technical Writing**：每个文档说明范围、非范围与目标读者。
- **Chromium**：最小可用文档、随代码更新、删除死文档、避免重复。

## 1. 原则

| 原则 | 落地方式 |
| --- | --- |
| Docs as Code | 文档与代码同仓、同 PR、同评审、同 CI |
| 单一来源 | 同一事实只在一处详细说明，其他位置链接 |
| 最小可用 | 先写能解决问题的短文档，不追求大而全 |
| 受众与范围 | 新文档说明目标读者、覆盖范围和不覆盖范围 |
| 当前性 | 行为变化必须同步文档；过期内容比缺失更糟 |
| 可验证 | 命令、路径、检查项优先；能用 `check.sh` 或 CI 验证 |
| 可发现 | 新增文档必须登记到 `docs/README.md` |
| 可删除 | 无历史价值的死文档直接删除，不做“以后再说”的保留 |

## 2. 文档组织

### 2.1 仓库角色

| 类型 | 位置 | 回答的问题 | 更新时机 |
| --- | --- | --- | --- |
| 入口 | `README.md` | 这是什么、怎么用 | 定位、用法、结构变化 |
| 契约 | `AGENTS.md` | AI 必须/禁止做什么 | 行为规则变化 |
| 贡献流程 | `CONTRIBUTING.md` | 人如何提交、评审、发布 | 流程变化 |
| 操作指南 | `docs/ci-cd.md` | 门禁怎么跑、失败怎么排查 | CI 或检查变化 |
| 配置参考 | `docs/configuration.md` | 每个配置文件负责什么 | 配置变化 |
| 背景说明 | `docs/ai-behavior.md` | 为什么这样设计、工具如何映射 | 机制或平台变化 |
| 治理规范 | `docs/governance.md` | 文档如何组织、评审、维护 | 治理规则变化 |
| 决策记录 | `docs/adr/NNNN-*.md` | 当时选了什么、放弃了什么 | 不可逆或有争议的选择 |
| 变更记录 | `CHANGELOG.md` | 使用者能感知到什么变化 | 每次合并 |

### 2.2 Diátaxis 映射

| 类型 | 目的 | 本仓库示例 |
| --- | --- | --- |
| Tutorial | 带读者完成一次学习 | `README.md` 快速开始 |
| How-to | 帮有经验的读者完成具体任务 | `docs/ci-cd.md` 排障、门禁运行 |
| Reference | 准确、完整、中立的查询信息 | `docs/configuration.md`、`AGENTS.md` 规则 |
| Explanation | 解释背景、取舍和“为什么” | `docs/ai-behavior.md`、ADR、本文 |

一个文档只服务一种主要目的。需要同时讲“怎么做”和“为什么”时，先写操作步骤，再链接到解释文档。

### 2.3 目录与命名

```text
docs/
├── README.md          # 索引与入口
├── governance.md      # 治理与组织（模板）
├── ai-behavior.md     # 背景说明（模板）
├── ci-cd.md           # 操作指南（模板）
├── configuration.md   # 配置参考（模板）
├── process/           # 通用流程：任务、评审
├── product/           # 本项目需求
│   ├── platform/      # 跨模块：租户、权益、管理端、合规
│   └── <module>/      # 模块需求与冷启动，当前 feedback/
├── design/            # 本项目设计
│   ├── engineering/   # 跨模块：代码规范、测试
│   ├── platform/      # 跨模块：认证、数据、API、前端壳
│   └── <module>/      # 模块专有设计，当前 feedback/
├── ops/               # 本项目运维
└── adr/               # 决策记录
    ├── README.md
    ├── template.md
    └── NNNN-*.md
```

根目录只保留模板治理文档；业务文档按主题归入子目录，不堆在根级。

- 文件名使用小写 kebab-case。
- 目录表达模块归属：`product/<module>/`、`design/<module>/`；跨模块内容放 `platform/`，工程规范放 `engineering/`。
- 模块内文件名固定为主题名：`requirements.md`、`launch.md`、`prompt.md`；不重复模块名前缀。新增主题先在模块 `README.md` 登记。
- 每个文档只有一个一级标题。
- 新增文档必须登记到所在目录的 `README.md`；`docs/README.md` 只登记目录入口。
- 站内链接使用相对路径；外链使用完整 URL。
- 模板规模较小时保持扁平目录；出现稳定主题、单文件过长或文档数量明显增长时再拆子目录，不预建空目录。

### 2.4 单一来源

- 入口信息放 `README.md`；行为约束放 `AGENTS.md`；解释放 `docs/`；取舍放 ADR。
- 同一事实只保留一个权威版本，其他位置用链接引用，不复制整段。
- 文档之间冲突时，按 `AGENTS.md` 第 0 节的优先级处理。

### 2.5 新项目落地路径

从模板派生时，项目会把现有 `docs/` 一起复制过去。因此重点不是“少复制”，而是**先继承基线，再做减法和替换，最后按需新增**。

| 来源 | 落地方式 |
| --- | --- |
| 从模板派生 | 复制基线 → 替换项目身份 → 删除不适用文档 → 按需新增 |
| 从空仓库开始 | 只建入口、契约和治理文档 → 按需新增 |

从模板派生时：

1. **替换入口与契约**：把 `README.md`、`AGENTS.md`、`CONTRIBUTING.md`、`SECURITY.md`、`CHANGELOG.md` 改成项目自己的信息。
2. **审查模板文档**：保留仍然成立的 `docs/governance.md`、`docs/ci-cd.md`、`docs/configuration.md`、`docs/ai-behavior.md` 与 `docs/process/`；删除不适用于该项目的示例、平台说明和历史记录。
3. **补项目文档**：平台规则放 `docs/product/platform/` 与 `docs/design/platform/`，模块需求与设计放 `docs/product/<module>/` 与 `docs/design/<module>/`；不再在根目录新建平铺文档。
4. **补决策**：需要解释取舍时新增 `docs/adr/NNNN-*.md`。
5. 每新增一篇文档都登记到所在目录的 `README.md` 与 `docs/README.md`，并运行 `bash scripts/check.sh docs`。

从空仓库开始时，只建当前真正需要的文档；不要预建空目录或空模板。

无论哪种方式，都不要把 `README.md`、`AGENTS.md` 的正文复制到第二份文档，也不要保留模板专属但项目不再适用的内容。

### 2.6 规模演进

| 规模 | 结构 | 索引方式 |
| --- | --- | --- |
| 小 | `docs/*.md` 扁平 | `docs/README.md` 直接索引每篇 |
| 中 | `docs/<category>/*.md` | `docs/README.md` 直接索引每篇 |
| 大 | `docs/<category>/README.md` + 子文档 | `docs/README.md` 只索引目录的 `README.md`；各层 README 逐级索引，`check.sh` 支持多级目录索引 |

拆分参考阈值：单文件明显过长、同一主题超过 5 篇，或读者/生命周期明显不同。拆分时同步更新相对链接、`docs/README.md` 和 `CHANGELOG.md`；旧路径只保留迁移说明，不复制正文。

## 3. 所有权与评审

| 角色 | 责任 |
| --- | --- |
| 维护者 | 对文档整体质量、索引和过期内容负责；个人模板默认由仓库所有者承担 |
| 作者 | 在代码或行为变更的同一个 PR 中更新相关文档 |
| 评审者 | 检查事实准确性、范围、链接、命令可执行性和索引登记 |
| AI 代理 | 可以起草、整理和检查文档，但不得替代人类确认事实与最终取舍 |

- 多人协作时用 `.github/CODEOWNERS` 指定文档负责人。
- 纯文档变更至少经过一次评审；个人仓库可由维护者自审，但仍必须通过 CI。
- AI 生成或大幅改写的文档必须由人类核对事实、命令和链接。

## 4. 变更与评审流程

1. **判定影响**：行为、配置、目录、CI 或公开接口变化时，文档属于同一个变更，不另开“以后补文档”的任务。
2. **选择类型与位置**：按第 2 节判断是操作指南、参考、解释还是 ADR。
3. **更新文档**：同步索引、交叉链接和相关命令。
4. **本地验证**：运行 `bash scripts/check.sh all`；文档改动至少运行 `docs` 与 `format`。
5. **评审**：确认事实、范围、示例可执行、链接可解析。
6. **记录变更**：使用者可感知的变化写入 `CHANGELOG.md`；长期取舍写入 ADR，同一主题优先修订现有 ADR，新主题或被推翻的决策才新增。

紧急修复可以先用最小文档解除阻塞，但必须在合并后 **7 天内**补完文档并创建跟踪 Issue；不得把缺口留成隐式债务。

## 5. 生命周期

| 状态 | 含义 | 处理方式 |
| --- | --- | --- |
| Draft | 尚未合并的草稿 | 留在 PR/分支，不进入索引 |
| Active | 当前有效 | 默认状态，无需额外标记 |
| Deprecated | 仍可查阅，但不再推荐 | 文首加 `> 状态：已废弃；替代：<链接>`，并修正指向它的链接 |
| Removed | 已无历史价值 | 直接删除；外部引用改为新文档或说明性占位 |

规则：

- 过期内容优先删除，而不是长期保留“可能有用”的段落。
- 有历史价值的决策不删除：同一主题的演进在原 ADR 追加「修订记录」；决策被推翻时新增 ADR 并标 `Superseded by NNNN`。
- 一个 ADR 只记录一个决策或一个模块；不得把多个模块的取舍堆进同一文件。
- 删除或替换文档时，必须同步更新 `docs/README.md` 索引和所有站内链接。
- 迁移期间需要保留旧路径时，旧文档只保留状态说明和替代链接，不复制正文。

## 6. 审计与指标

### 每次变更

- `bash scripts/check.sh all`
- 文档结构、索引、站内链接、ADR 索引和文本格式通过
- 新增或修改的命令实际执行过

### 每季度

- 检查索引中的文档是否仍存在、是否仍属于当前结构
- 检查 ADR 是否按主题重复新增，或一个文件混入多个独立决策/模块
- 抽查命令、版本号、平台行为和外链
- 删除过期内容，合并重复说明
- 检查 `AGENTS.md`、`CONTRIBUTING.md` 与 `docs/` 是否仍有冲突

### 可观察指标

| 指标 | 目标 |
| --- | --- |
| 未登记文档 | 0，由 `check.sh docs` 拦截 |
| 站内死链 | 0，由 `check.sh docs` 与 lychee 拦截 |
| 过期命令/版本 | 季度审计后无已知过期项 |
| 文档缺口 | 有明确 Issue 和负责人，不静默遗留 |
| 重复说明 | 同一事实只有一个权威位置 |

## 7. 例外

- 安全事件或紧急修复可以先处理问题，再补文档；必须留下 Issue。
- 外部平台行为无法本地验证时，标注“未验证”、访问日期和复核条件。
- 个人模板不强制独立技术写作者；维护者承担最终评审责任。

## 8. 参考

- Write the Docs：Docs as Code：<https://www.writethedocs.org/guide/docs-as-code/>
- Write the Docs：Documentation principles：<https://www.writethedocs.org/guide/writing/docs-principles/>
- Diátaxis：Start here：<https://diataxis.fr/start-here/>
- GitLab：Documentation Style Guide：<https://docs.gitlab.com/development/documentation/styleguide/>
- GitLab：Documentation workflows：<https://docs.gitlab.com/development/documentation/workflow/>
- GitLab：Topic types：<https://docs.gitlab.com/development/documentation/topic_types/>
- Google：Technical Writing One — Documents：<https://developers.google.com/tech-writing/one/documents>
- Google：Organizing large documents：<https://developers.google.com/tech-writing/two/large-docs>
- Chromium：Documentation Best Practices：<https://chromium.googlesource.com/chromium/src/+/main/docs/documentation_best_practices.md>

访问日期：2026-10-31。
