# 设计文档

按跨模块契约与功能模块组织：跨模块规则放 `engineering/` 与 `platform/`，模块专有设计放模块目录。

- **类型**：项目设计
- **覆盖范围**：本项目的工程规范、系统契约与模块设计。
- **不覆盖范围**：与业务无关的工作方式见 [流程规范](../process/README.md)；需求见 [产品文档](../product/README.md)；取舍理由见 [ADR](../adr/README.md)。

## 结构

```text
design/
├── engineering/        # 跨模块：代码规范、测试策略
├── platform/           # 跨模块：认证、数据模型、API 契约、前端壳
└── feedback/           # 当前模块：课后反馈
    ├── README.md
    └── prompt.md
```

新增模块时在 `design/` 下新建同名目录；模块的表、端点、页面若属于全系统共享契约，写在 `platform/` 并在模块 `README.md` 中给出章节映射。

## 文档

| 目录 | 内容 |
| --- | --- |
| [工程规范](engineering/README.md) | 代码规范与测试策略 |
| [平台契约](platform/README.md) | 认证、数据模型、API 契约与前端壳 |
| [课后反馈模块](feedback/README.md) | 模块设计入口与 Prompt 约束 |

### 工程规范

| 文档 | 内容 |
| --- | --- |
| [代码规范](engineering/code-standards.md) | 工作区结构、依赖方向、模块分层、规模上限、可见性、错误处理、注释标准 |
| [测试策略](engineering/testing.md) | 必测清单、测试分层、集成测试写法、CI 分工 |

### 平台契约

| 文档 | 内容 |
| --- | --- |
| [认证与安全](platform/authentication.md) | 会话、注册、密码、CSRF、限流、双因素、分享链接 |
| [数据库设计](platform/database.md) | 表结构、索引、行级安全、删除策略、配额扣减、迁移规则 |
| [API 契约](platform/api-contract.md) | 路由分组、端点、错误码、分页、幂等、契约生成 |
| [前端架构](platform/frontend.md) | 共享边界、状态管理、离线草稿、流式渲染、微信兼容、性能预算 |

### 模块设计

| 文档 | 内容 |
| --- | --- |
| [课后反馈](feedback/README.md) | 模块设计入口、Prompt 约束与共享契约章节映射 |

## 评审关注点

评审时按关注点查阅对应章节：

| 关注点 | 位置 |
| --- | --- |
| 租户隔离与越权 | [数据库设计](platform/database.md) 第 5 节、[测试策略](engineering/testing.md) 必测清单 |
| 认证、会话与 CSRF | [认证与安全](platform/authentication.md) |
| 接口形状与错误码 | [API 契约](platform/api-contract.md) |
| 代码组织与规模 | [代码规范](engineering/code-standards.md) 第 2、4 节 |
| 前端交互与兼容 | [前端架构](platform/frontend.md) |
| 反馈质量与 Prompt | [模块设计](feedback/README.md)、[Prompt 设计](feedback/prompt.md) |

## 命名

- 跨模块目录：`engineering/`、`platform/`。
- 模块目录：使用模块名，如 `feedback/`；模块内主题文件固定为 `prompt.md`，需求与冷启动在 `product/<module>/`。
- 文件名小写 kebab-case；目录已经表达模块归属，不再给文件名加模块前缀。

## 单一来源

以下事实以代码或生成物为准，本目录只记录原则与约定：

| 事实 | 权威来源 |
| --- | --- |
| 表结构与索引 | 迁移文件 |
| 接口字段 | `openapi.json` |
| 错误码枚举 | 后端类型定义 |

文档与生成物不一致时，以生成物为准并修正文档。
