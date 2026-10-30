# 文档索引

`docs/` 是本模板的**规范来源**。文档如何分类、命名、评审、废弃和审计，见 [文档治理与组织](governance.md)。

## 目录

```text
docs/
├── README.md          # 本文件：索引与入口
├── governance.md      # 文档治理、组织、生命周期与审计
├── ai-behavior.md     # AI 指令文件生态、映射与审计
├── ci-cd.md           # GitHub 门禁、运行方式与排障
├── configuration.md   # 各配置文件职责、可选/必需、维护方式
└── adr/               # 架构与规范决策记录
    ├── README.md
    ├── template.md
    └── NNNN-*.md
```

## 索引

### 核心文档

- [文档治理与组织](governance.md)：文档类型、所有权、评审、生命周期与审计。
- [AI 行为与指令文件](ai-behavior.md)：`AGENTS.md` 的定位、工具映射、冲突处理。
- [CI/CD 与质量门禁](ci-cd.md)：GitHub Actions、本地检查与排障。
- [项目配置说明](configuration.md)：每个配置文件的职责、可选性与维护方式。

### 决策记录

- [ADR 索引与流程](adr/README.md)
- [ADR 模板](adr/template.md)

## 新增文档

1. 按 [文档治理与组织](governance.md) 判断类型和位置。
2. 在本文件登记；否则 `scripts/check.sh docs` 会失败。
3. 运行 `bash scripts/check.sh docs`。
