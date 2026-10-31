# 文档索引

`docs/` 是本项目的**规范来源**。文档如何分类、命名、评审、废弃和审计，见 [文档治理与组织](governance.md)。

## 目录

```text
docs/
├── README.md              # 本文件：索引与入口
├── governance.md          # 模板治理：文档分类、生命周期与审计
├── ai-behavior.md         # 模板治理：AI 指令文件生态与映射
├── ci-cd.md               # 模板治理：门禁矩阵与排障
├── configuration.md       # 模板治理：配置文件职责
├── product/               # 产品侧
│   ├── README.md
│   ├── requirements.md
│   └── prompt-design.md
├── engineering/           # 工程侧
│   ├── README.md
│   ├── database.md
│   ├── api-contract.md
│   ├── authentication.md
│   └── testing.md
├── ops/                   # 运维侧
│   ├── README.md
│   └── deployment.md
└── adr/                   # 决策记录
    ├── README.md
    ├── template.md
    └── NNNN-*.md
```

## 索引

### 模板治理

- [文档治理与组织](governance.md)：文档类型、所有权、评审、生命周期与审计。
- [AI 行为与指令文件](ai-behavior.md)：`AGENTS.md` 的定位、工具映射、冲突处理。
- [CI/CD 与质量门禁](ci-cd.md)：GitHub Actions、本地检查与排障。
- [项目配置说明](configuration.md)：每个配置文件的职责、可选性与维护方式。

### 产品

- [产品文档索引](product/README.md)：需求说明与 Prompt 质量设计。

### 工程

- [工程文档索引](engineering/README.md)：数据库、API 契约、认证安全与测试策略。

### 运维

- [运维文档索引](ops/README.md)：部署、备份恢复与可观测性。

### 决策记录

- [ADR 索引与流程](adr/README.md)

## 分层规则

| 层 | 放什么 | 回答的问题 |
| --- | --- | --- |
| 根目录 | 模板治理文档，与具体业务无关 | 代理与协作规则如何运作 |
| `product/` | 需求、用户流程、内容设计 | 做什么、给谁 |
| `engineering/` | 数据模型、接口、安全、质量 | 怎么实现、怎么验证 |
| `ops/` | 部署、备份、监控、故障处理 | 怎么运行、怎么恢复 |
| `adr/` | 不可逆或有争议的取舍 | 为什么这样选 |

根目录只保留模板治理文档。业务文档一律归入 `product/`、`engineering/`、`ops/` 之一；无法归类时先确认是否真的需要单独成文。

## 新增文档

1. 判断归属：模板治理放根目录，业务文档放对应子目录。
2. 在本文件登记子目录的 `README.md`；子目录内新增的文件在本目录的 `README.md` 登记。
3. 运行 `bash scripts/check.sh docs`。

`check.sh` 支持目录级索引：子目录的 `README.md` 被本文件索引，且该 `README.md` 索引了目录内的文件，即视为已登记。
