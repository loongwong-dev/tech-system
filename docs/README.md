# 文档索引

`docs/` 是本项目的**规范来源**。文档如何分类、命名、评审、废弃和审计，见 [文档治理与组织](governance.md)。

## 目录

```text
docs/
├── README.md              # 本文件：索引与入口
├── governance.md          # 通用规范：文档分类、生命周期与审计
├── ai-behavior.md         # 通用规范：AI 指令文件生态与映射
├── ci-cd.md               # 项目运维：门禁矩阵与排障
├── configuration.md       # 项目运维：配置文件职责
├── process/               # 通用流程规范
│   ├── README.md
│   ├── issue-workflow.md
│   └── code-review.md
├── product/               # 本项目需求
│   ├── README.md
│   ├── platform/
│   │   ├── README.md
│   │   └── requirements.md
│   └── feedback/
│       ├── README.md
│       ├── requirements.md
│       └── launch.md
├── design/                # 本项目设计
│   ├── README.md
│   ├── engineering/
│   │   ├── README.md
│   │   ├── code-standards.md
│   │   └── testing.md
│   ├── platform/
│   │   ├── README.md
│   │   ├── authentication.md
│   │   ├── database.md
│   │   ├── api-contract.md
│   │   └── frontend.md
│   └── feedback/
│       ├── README.md
│       └── prompt.md
├── ops/                   # 本项目运维
│   ├── README.md
│   └── deployment.md
└── adr/                   # 决策记录
    ├── README.md
    ├── template.md
    └── NNNN-*.md
```

## 索引

### 治理与运维

- [文档治理与组织](governance.md)：文档类型、所有权、评审、生命周期与审计。
- [AI 行为与指令文件](ai-behavior.md)：`AGENTS.md` 的定位、工具映射、冲突处理。
- [CI/CD 与质量门禁](ci-cd.md)：GitHub Actions、本地检查与排障。
- [项目配置说明](configuration.md)：每个配置文件的职责、可选性与维护方式。

### 通用流程

- [流程规范](process/README.md)：任务管理、评审与合并的通用做法。

### 本项目

- [产品文档](product/README.md)：平台规则、课后反馈模块需求与冷启动计划。
- [设计文档](design/README.md)：工程规范、平台契约与课后反馈模块设计。
- [运维文档](ops/README.md)：部署、备份恢复与可观测性。

### 决策记录

- [ADR 索引与流程](adr/README.md)

## 分层规则

| 层 | 放什么 | 是否随项目变化 |
| --- | --- | --- |
| 根目录 | 治理与运维文档 | 部分：治理规则稳定，CI 与配置随项目 |
| `process/` | 与业务无关的工作方式 | 否，可整体复用 |
| `product/` | 平台规则与各模块需求 | 是 |
| `design/` | 工程规范、平台契约与模块设计 | 是 |
| `ops/` | 部署、备份、监控 | 是 |
| `adr/` | 不可逆或有争议的取舍 | 是，按决策累积 |

判断归属只需问一句：**换一个项目，这份内容还成立吗？** 成立放根目录或 `process/`，不成立放 `product/`、`design/`、`ops/`。

## 新增文档

1. 判断归属：先答上面那个问题，再选目录。
2. 在上一级 `README.md` 登记子目录的 `README.md`；子目录内新增的文件在该目录的 `README.md` 登记。
3. 运行 `bash scripts/check.sh docs`。

`check.sh` 支持多级目录索引：子目录的 `README.md` 被上一层 `README.md` 索引，且该 `README.md` 索引了目录内的文件，即视为已登记。
