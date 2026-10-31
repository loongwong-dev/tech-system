# 课后反馈模块设计

反馈模块的专有设计入口。跨模块契约不在这里复制，只给章节位置。

- **类型**：项目设计
- **覆盖范围**：Prompt 约束、输出校验与回归流程；共享契约的章节映射。
- **不覆盖范围**：需求见 [模块需求](../../product/feedback/requirements.md)；平台契约见 [平台设计](../platform/README.md)。

## 文档

| 文档 | 内容 |
| --- | --- |
| [Prompt 设计](prompt.md) | 输入契约、提示词分层、硬约束、校验规则、模板兜底、金标集与回归 |

## 共享契约位置

| 内容 | 位置 |
| --- | --- |
| 模块需求与状态机 | [需求说明](../../product/feedback/requirements.md) 第 3 节 |
| 反馈相关表与 RLS | [数据库设计](../platform/database.md) 第 3、5 节 |
| 反馈与分享端点 | [API 契约](../platform/api-contract.md) 第 2、7、8 节 |
| 录入与审核页面 | [前端架构](../platform/frontend.md) 第 3、4、5 节 |
| 图片渲染 | [ADR-0003](../../adr/0003-typst-renderer.md) |

新增模块专有设计时，在本目录新增文件并登记到上表或本文档表；共享内容仍以 `platform/` 为唯一来源。
