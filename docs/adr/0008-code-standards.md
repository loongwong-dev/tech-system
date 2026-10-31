# ADR-0008：代码规范以可检查规则落地

- 状态：Accepted
- 日期：2026-10-31
- 决策者：仓库维护者
- 相关：[代码规范](../engineering/code-standards.md)、[AGENTS.md](../../AGENTS.md)

## 背景

技术选型已收敛，但缺少工程约束：目录怎么组织、模块边界在哪、文件多大算过大，都没有成文规则。单人开发加 AI 辅助时，这一问题的后果被放大——AI 倾向于把新功能追加到已有文件，几个月后会出现承载多个职责的大文件，评审时也说不清改动影响了什么。

- 约束 1：单人维护，规则必须能自动检查，不能依赖团队纪律。
- 约束 2：规则要能用现有工具执行，不引入自定义静态分析。
- 不做什么：不写风格口号，不做逐条人工审查的长清单。

## 决策

在 `docs/engineering/code-standards.md` 定义规则，分为两类：

1. **结构规则**由编译期与目录约定保证：crate 平铺、依赖单向、模块内按 model / repository / service / routes 分层。
2. **规模规则**由工具保证：文件行数用 `scripts/check-size.sh`，函数与参数用 clippy，依赖卫生用 cargo-machete 与 cargo-deny。

`check.sh all` 纳入 `check-size.sh`，作为新的强制门禁。

硬上限取 500 行：低于社区常见的一次性重写阈值，高于绝大多数单职责文件的实际长度。超过时允许用 `code-size:allow` 显式豁免，使例外留在代码里可被检索，而不是藏在讨论中。

## 后果

### 正面

- 「这个文件该不该拆」有了可执行的判据，而不是凭感觉。
- 新增门禁不依赖外部服务，纯 bash，与仓库现有检查一致。
- 豁免机制让必要的例外可见、可审计。

### 负面 / 代价

- 行数上限会误伤测试密集的文件；已通过排除测试代码缓解。
- 前端与后端共用同一阈值，前端的 300 行软上限实际更严格。
- 多一个脚本要维护。

### 风险与缓解

- 风险：规则流于形式，500 行内依然耦合。
  缓解：结构与规模分开规定；评审清单中「一个变更理由」是主判据，行数只是触发器。
- 风险：`code-size:allow` 被滥用。
  缓解：豁免必须写明理由，评审可见；数量异常时可统计。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 只写口头规范 | 零成本 | 无法执行，AI 与人都容易忽视 | 违背「规则必须可检查」 |
| 引入专用 lint 工具 | 检查更细 | 需要额外依赖与维护，社区方案零散 | 现有工具已足够 |
| 用 clippy 的 `too_many_lines` 代替脚本 | 少一个脚本 | 只能检查函数，管不到文件 | 覆盖不到核心诉求 |

## 验证方式

- 命令或检查项：`bash scripts/check.sh size`；构造一个超过 500 行的文件应失败，加豁免注释后通过。
- 观察指标：超过软上限 300 行的文件数量。
- 复核时间：首次实现完成后一个月，确认阈值是否过严或过松。

## 参考

- Microsoft Pragmatic Rust Guidelines：<https://microsoft.github.io/rust-guidelines/guidelines/project>（访问日期：2026-10-31）
- Cargo 清单的 lints 配置：<https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section>（访问日期：2026-10-31）
- Clippy lint 列表：<https://doc.rust-lang.org/clippy/>（访问日期：2026-10-31）
- cargo-deny：<https://github.com/EmbarkStudios/cargo-deny>（访问日期：2026-10-31）
