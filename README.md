# 教培教师 AI 反馈平台

面向教培老师的课后反馈工具：按课次批量打标签，AI 润色成文，老师审核后生成图片或链接发给家长。

## 定位

教培老师课后要逐个向家长反馈学生学习情况，一个班十几名学生，逐条撰写通常要一小时以上。本产品把这件事压缩到十分钟以内。

三个刻意选择：

| 选择 | 理由 |
| --- | --- |
| 面向教师个人 | 机构会自建或采购整体方案，个人教师才是能自助转化的群体 |
| 面向教培，而非公立校 | 公立校走财政与集采，个人付费路径不成立 |
| 卖工作流，而非生成能力 | 单点生成已被通用工具覆盖，壁垒在批量、审核与分享闭环 |

## 当前状态

| 项 | 状态 |
| --- | --- |
| 需求与验收标准 | 已完成 |
| 技术方案与架构决策 | 已完成 |
| 代码实现 | 进行中（M0 基座） |
| 备案、主体与合规确认 | 待核实 |

## v1 范围

包含：邀请码注册、学员与班级、课次批量录入、AI 润色与模板生成、教师审核、图片与链接分享、配额与权益、运营管理端。

不包含：课件资源市场、在线支付、家长账号、短信代发、趋势图表、语音反馈、小程序与原生 App。

完整范围与验收标准见 [平台需求](docs/product/platform/requirements.md) 与 [反馈模块需求](docs/product/feedback/requirements.md)。

## 技术栈

| 层 | 选择 |
| --- | --- |
| 后端 | Rust、tokio、axum、sqlx |
| 数据库 | PostgreSQL，应用层租户约束 + 行级安全策略 |
| 任务队列 | PostgreSQL 表 + `SKIP LOCKED`，自建 |
| 图片渲染 | typst CLI |
| 前端 | Vite、React、TanStack Query、shadcn/ui |
| 部署 | 单台服务器 + Docker Compose + Caddy |

取舍理由记录在 [ADR 索引](docs/adr/README.md)。

## 仓库结构

```text
crates/          Rust 后端（workspace 骨架已建立）
  db/            连接池、迁移与仓储基元
    migrations/  数据库迁移（sqlx）
deploy/          部署编排与数据库角色引导
  postgres/      PostgreSQL 角色与权限引导
apps/            前端应用（workspace 骨架已建立）
  web/           C 端
  admin/         管理端
packages/        共享包（workspace 骨架已建立）
  ui/            设计系统
  api-types/     OpenAPI 生成物
docs/            文档：process / product / design / ops / adr
scripts/         检查脚本
```

`crates/` 已建立 8 个 crate 的 Rust workspace，数据库迁移位于 `crates/db/migrations/`；`apps/` 与 `packages/` 已建立 Bun workspace 骨架。

## 文档

从 [文档索引](docs/README.md) 进入，按层组织：

| 层 | 入口 | 回答的问题 |
| --- | --- | --- |
| 通用流程 | [process](docs/process/README.md) | 任务怎么管、怎么评审 |
| 产品 | [product](docs/product/README.md) | 平台规则与课后反馈模块做什么 |
| 设计 | [design](docs/design/README.md) | 工程规范、平台契约与模块怎么实现 |
| 运维 | [ops](docs/ops/README.md) | 怎么运行、怎么恢复 |
| 决策 | [adr](docs/adr/README.md) | 为什么这样选 |

## 质量门禁

```bash
bash scripts/check.sh all       # 全量
bash scripts/check.sh docs      # 文档结构、索引、ADR、站内链接
bash scripts/check.sh format    # 换行、空白、Tab
bash scripts/check.sh size      # 源码文件行数上限
bash scripts/check.sh commits   # Conventional Commits
bash scripts/check.sh secrets   # 密钥基线扫描

bun install                     # 根目录安装前端依赖
bun run build                   # 构建 web 与 admin
bun run test                    # 前端 Vitest
bun run size                    # web/admin 首屏 JS gzip 预算
```

| 检查 | 本地 | GitHub |
| --- | --- | --- |
| 文档结构、H1、索引、ADR 命名 | ✅ | ✅ |
| 站内链接 | ✅ | ✅ |
| 外链 | 可选（lychee） | ✅ |
| Markdown 完整规则 | 可选（markdownlint） | ✅ |
| 源码文件行数 | ✅ | CI |
| 提交信息 | ✅ | ✅ |
| 密钥扫描 | 基线 | gitleaks |

失败处理与排障见 [CI/CD 与质量门禁](docs/ci-cd.md)。

## 协作约定

- AI 代理的行为契约见 [AGENTS.md](AGENTS.md)。
- 任务从 Issue 来，进度由标签派生，改动按 `(#N)` 回溯，见 [Issue 驱动开发流程](docs/process/issue-workflow.md)。
- 合并前必须通过评审，AI 不得自行合并，见 [代码评审](docs/process/code-review.md)。
- 提交与评审流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。
- 代码组织、模块边界与规模上限见 [代码规范](docs/design/engineering/code-standards.md)。
- 使用者可感知的变更记录在 [CHANGELOG.md](CHANGELOG.md)。

## 已知限制

- 代码尚未开始，当前无法运行产品。
- ICP 备案主体类型与生成式 AI 服务备案口径待核实，上线前必须确认。
- 单机部署，无高可用；可用性目标 99%，故障依赖重启与备份恢复。
- 前端不做 SSR 与小程序，移动端体验上限低于原生。

## 许可证

本项目采用 [Apache License 2.0](LICENSE)。治理基线来自 `agent-dev-charter` 模板，已按本项目改造。
