# ADR-0013：项目特有的代码与测试规范归入设计文档

- 状态：Superseded by [ADR-0014](0014-module-docs-layout.md)
- 日期：2026-10-31
- 决策者：仓库维护者
- 相关：[代码规范](../design/engineering/code-standards.md)、[测试策略](../design/engineering/testing.md)、[ADR-0012](0012-process-design-split.md)

## 背景

ADR-0012 把 `process/` 定义为「换一个项目仍然成立」的通用规范，但 `code-standards.md` 与 `testing.md` 的内容仍绑定本项目：

- 工作区树、crate 名、领域词汇、门禁命令、P0 用例与工具链都是本项目取值。
- 读者在 `process/` 看到项目专有规则，仍需判断哪些可复用，正是 ADR-0012 要消除的混淆。
- ADR-0012 的「负面 / 代价」已记录这一残留，但没有给出归属。

- 约束 1：分类判据保持一句话：换一个项目，这份内容还成立吗。
- 约束 2：规则编号与内容不变，只调整归属与链接。
- 不做什么：不把两份文档再拆成通用与项目两半，避免同一事实维护两份。

## 决策

1. `docs/process/code-standards.md` 移至 `docs/design/engineering/code-standards.md`。
2. `docs/process/testing.md` 移至 `docs/design/engineering/testing.md`。
3. 两份文档的「类型」改为项目设计；`process/` 只保留与业务无关的 `issue-workflow.md` 与 `code-review.md`。
4. 新增项目特有规则时，先写入对应的 `design/` 文档或 CI，不写入 `process/`。
5. `process/` 文档可以链接项目文档作为「不覆盖范围」，但不复制其规则。

## 后果

### 正面

- `process/` 可整体复制到其他项目，不再需要逐条判断哪些示例要替换。
- 代码与测试规则随项目演进，不需要维持「通用」措辞。
- 文档头部、目录路径与内容三者一致。

### 负面 / 代价

- `design/` 目录变大，需要靠 `README.md` 的索引维持可读性。
- 旧链接需要迁移；Issue 模板、README、CHANGELOG 中的旧路径必须同步。

### 风险与缓解

- 风险：项目特有规则再次被写进 `process/`。
  缓解：新增文档时先回答分类判据；评审检查 `process/` 中是否出现 crate 名、领域词或项目命令。
- 风险：移动文件导致站内链接断裂。
  缓解：`bash scripts/check.sh docs` 校验全部本地链接。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 保留在 `process/` 并标注示例 | 零迁移成本 | 读者仍需逐条判断可复用性 | 混淆没有消除 |
| 拆成通用与项目两份 | 边界最清晰 | 同一规则维护两处，容易漂移 | 违背单一来源 |
| 只改「类型」字段不改路径 | 改动最小 | 路径仍暗示通用规范 | 与目录语义冲突 |

## 验证方式

- 命令或检查项：`bash scripts/check.sh docs`；`docs/process/` 中不出现项目 crate 名、领域词与项目专用命令。
- 观察指标：新增文档时是否需要反复讨论归属。
- 复核时间：下一次新增工程规范时。

## 参考

- [ADR-0012：文档按性质拆分通用流程与本项目设计](0012-process-design-split.md)
- Diátaxis：<https://diataxis.fr/start-here/>（访问日期：2026-10-31）
