# API 契约

后端对外接口的分组、命名、错误模型与契约生成流程。前端与后端都以本文为准。

- **类型**：项目设计
- **覆盖范围**：路由分组、端点、请求响应约定、错误码、分页、幂等、流式与批量、契约生成。
- **不覆盖范围**：鉴权细节见 [认证与安全](authentication.md)，表结构见 [数据库设计](database.md)。
- **状态**：已定，接口变更时同步更新。

## 1. 路由分组

| 分组 | 前缀 | 鉴权 | 读取的会话 |
| --- | --- | --- | --- |
| 认证 | `/auth/*` | 无 | 视端点而定 |
| C 端 | `/api/v1/*` | C 端 Cookie + 租户上下文 | `sid` |
| 管理端 | `/admin/api/v1/*` | 管理端 Cookie + 平台角色 | `asid` |
| 公开 | `/s/{token}` | 无 | 不读会话 |
| 运维 | `/healthz`、`/readyz` | 无 | 不读会话 |

两个会话 Cookie 互不通用：教师拿着合法的 `sid` 访问管理端接口时，服务端不会读取它，直接返回 401。

## 2. 端点

| 方法 | 路径 | 说明 |
| --- | --- | --- |
| POST | `/auth/register` | 邮箱、密码、邀请码 |
| POST | `/auth/login` | 返回会话 Cookie |
| POST | `/auth/logout` | 吊销当前会话 |
| POST | `/auth/tenant` | 切换租户并轮换 token |
| GET | `/auth/tenants` | 当前会话可切换的租户列表 |
| GET | `/api/v1/me` | 当前用户、租户、角色、配额余量 |
| GET | `/api/v1/students` | 当前租户未归档学员 |
| POST | `/api/v1/students` | 新建学员 |
| PUT | `/api/v1/students/{id}` | 更新学员 |
| POST | `/api/v1/students/{id}/archive` | 归档学员 |
| DELETE | `/api/v1/students/{id}` | 删除学员，请求体提交姓名确认 |
| GET | `/api/v1/classes` | 当前租户班级列表 |
| POST | `/api/v1/classes` | 新建班级 |
| PUT | `/api/v1/classes/{id}` | 更新班级 |
| POST | `/api/v1/classes/{id}/archive` | 归档班级 |
| DELETE | `/api/v1/classes/{id}` | 删除空班级 |
| GET | `/api/v1/lessons?from=&to=&limit=&cursor=` | 课次列表，游标分页 |
| POST | `/api/v1/lessons` | 新建课次 |
| GET | `/api/v1/lessons/{id}/feedbacks` | 该课次全部反馈 |
| PUT | `/api/v1/feedbacks/{id}` | 保存草稿四个字段，客户端生成 id 的幂等 upsert |
| POST | `/api/v1/feedbacks/{id}/generations` | 单份生成，流式返回 |
| POST | `/api/v1/lessons/{id}/generations` | 批量生成，返回 `jobId` |
| GET | `/api/v1/jobs/{id}` | 批量进度 |
| POST | `/api/v1/feedbacks/{id}/approval` | 教师确认 |
| POST | `/api/v1/feedbacks/{id}/shares` | 生成图片与链接 |
| DELETE | `/api/v1/shares/{id}` | 吊销分享物 |
| GET | `/admin/api/v1/tenants` | 租户列表 |
| GET | `/admin/api/v1/tenants/{id}` | 租户详情 |
| POST | `/admin/api/v1/tenants/{id}/entitlements` | 开通增值包 |
| POST | `/admin/api/v1/tenants/{id}/status` | 冻结或解冻 |
| GET | `/admin/api/v1/invite-codes` | 邀请码列表 |
| POST | `/admin/api/v1/invite-codes` | 生成邀请码 |
| GET | `/admin/api/v1/audit-logs` | 审计查询 |

命名规则：

- 状态流转用子资源加名词（`/generations`、`/approval`、`/shares`），不用动词路径。
- 资源集合用复数。
- 路径全小写，多词用连字符。
- 归档班级不再接受新课次；`POST /api/v1/lessons` 返回 404。

## 3. 请求与响应约定

| 项 | 约定 |
| --- | --- |
| 字段命名 | JSON 用 `camelCase`；数据库 `snake_case`，由序列化层转换 |
| 成功响应 | 直接返回资源本体，不套 `code`/`data`/`msg` |
| 时间 | ISO 8601 UTC 字符串；纯日期用 `YYYY-MM-DD` |
| ID | UUID 字符串 |
| 空值 | 省略字段还是返回 `null`：统一返回 `null` |
| 内容类型 | 变更接口只接受 `application/json` |
| 请求 ID | 所有响应带 `x-request-id`，由服务端生成并回写 |

## 4. 错误模型

统一使用 RFC 9457 Problem Details，`Content-Type: application/problem+json`。

```json
{
  "type": "https://example.com/errors/quota-exhausted",
  "title": "AI 配额已用尽",
  "status": 409,
  "detail": "本月剩余 0 份，模板生成仍可继续使用。",
  "instance": "/api/v1/feedbacks/8f2c/generations",
  "code": "QUOTA_EXHAUSTED",
  "errors": []
}
```

前端只判断 `code`，不解析 `detail`。`detail` 面向人，`code` 面向程序。

| code 前缀 | 含义 | HTTP |
| --- | --- | --- |
| `AUTH_` | 未登录、会话过期、凭据错误 | 401 |
| `FORBIDDEN_` | 角色或租户不匹配 | 403 |
| `NOT_FOUND` | 资源不存在或不属于当前租户 | 404 |
| `VALIDATION_` | 字段校验失败，`errors` 给出字段与原因 | 422 |
| `CONFLICT_` | 状态不允许、重复提交 | 409 |
| `QUOTA_` | 配额相关 | 409 |
| `RATE_LIMIT_` | 限流 | 429 |
| `INTERNAL_` | 未预期错误 | 500 |

**跨租户访问一律返回 404。** 403 会暴露「该 ID 存在」这一事实。

## 5. 分页

列表一律游标分页：

```text
GET /api/v1/lessons?limit=20&cursor=<opaque>
→ { "items": [...], "nextCursor": "<opaque>|null" }
```

- 默认 20，上限 100。
- 游标是不透明字符串，客户端不得解析。
- 不使用 offset：课次会持续新增，offset 会漏数据与重复。
- 游标基于排序键，不提供快照一致性：翻页期间新增或删除的记录可能不落在后续页。

## 6. 幂等

所有 `POST` 接受可选 `Idempotency-Key` 头：

- 同一租户、同一 key、同一路径在 24 小时内重复提交，返回首次结果。
- 用于防止重复点击产生两份生成任务或两条分享物。
- 存储于 `jobs` 的 `dedupe_key` 或独立幂等表，按端点选择。

## 7. 反馈生成、草稿写入与删除

| 场景 | 接口 | 传输 | 前端处理 |
| --- | --- | --- | --- |
| 单份生成 | `POST /api/v1/feedbacks/{id}/generations` | `text/event-stream` | 边接收边渲染 |
| 批量生成 | `POST /api/v1/lessons/{id}/generations` | JSON 返回 `jobId` | 轮询或订阅进度 |

单份必须流式：教师是盯着等结果的，首字超过 3 秒就会放弃。

单份 SSE 事件：

```text
event: delta
data: 课堂投入度高…

event: done
data: {"feedbackId":"…","text":"…","generatedBy":"template","promptVersion":"template-v1","reused":false}
```

- `delta` 是正文分块；`done` 是完整结果，前端以 `done.text` 为准。
- 未接入模型时 `generatedBy` 为 `template`；接入 AI 后为 `ai`，事件格式不变。
- 相同 `Idempotency-Key` 返回上次正文，`done.reused` 为 `true`，不增加重新生成次数。
- 生成失败、跨租户或重新生成超过 3 次时，在开始 SSE 前返回 Problem Details。

批量返回任务而不是长连接：教师可以离开页面，任务在服务端继续。`POST /api/v1/lessons/{id}/generations` 返回 `202 Accepted`，响应与进度查询一致：

```json
{ "jobId": "…", "status": "pending", "total": 15, "done": 0, "failed": 0 }
```

`GET /api/v1/jobs/{id}` 按当前租户读取，跨租户返回 404。`status` 取 `pending`、`running`、`succeeded`、`dead`、`cancelled`；`done` 在任务成功后等于 `total`，`failed` 在死信时等于 `total`。批量端点接受 `Idempotency-Key`，同一课次与同一个 key 返回同一任务。

## 7.1 学员删除

`DELETE /api/v1/students/{id}` 请求体必须是当前姓名：

```json
{ "name": "小明" }
```

姓名不匹配返回 `422` 与 `code=VALIDATION_NAME_MISMATCH`。删除在单个租户事务内完成：统计反馈数、写 `audit_logs`（`student.deleted`）、删除学员；`feedbacks` 与 `share_artifacts` 由外键级联删除，原分享 token 随分享物失效。`POST /api/v1/students/{id}/archive` 只归档，不删除反馈。

`POST /api/v1/classes/{id}/archive` 只归档班级，保留课次与历史数据；归档后不再出现在默认列表。`DELETE /api/v1/classes/{id}` 在删除前锁定班级并检查课次；仍有课次时返回 `409 CONFLICT_CLASS_HAS_LESSONS`，不删除课次。

## 7.2 反馈草稿读写

`GET /api/v1/lessons/{id}/feedbacks` 返回 `{ "items": [...] }`：该课次**全部**反馈，包含 `generated`、`approved`、`shared` 等非草稿状态，按 `studentId` 升序排列，没有反馈时 `items` 为空数组。课次不属于当前租户返回 404，课次 ID 不是 UUID 返回 422。

本端点不分页、不返回 `nextCursor`，是第 5 节的例外：录入页需要一节课的完整快照。

`PUT /api/v1/feedbacks/{id}` 保存四个可编辑字段，语义是**幂等 upsert**：路径 `id` 由客户端生成，不使用第 6 节的 `Idempotency-Key`（幂等性来自 `(lesson_id, student_id)` 唯一约束）。取舍见 [ADR-0017](../../adr/0017-feedback-draft-upsert.md)。

检查顺序固定，先失败的先返回：请求体字段校验（422）→ `lessonId`/`studentId` 归属（404）→ 路径 `id` 归属（409）→ 命中行状态（409）→ 写入（200）。

| 情况 | 行为 |
| --- | --- |
| 主键未占用，且 `(lessonId, studentId)` 无记录 | 插入，响应 `id` 等于请求 id，初始状态 `draft` |
| 主键已指向同一份草稿，且该行状态为 `draft` | 覆盖四个可编辑字段，返回同一行 |
| `(lessonId, studentId)` 已有草稿（id 不同），且该行状态为 `draft` | 更新该行并返回它的真实 `id`，客户端以后端返回值为准 |
| 命中行不是 `draft`（如 `generated`） | 409 `CONFLICT_INVALID_TRANSITION` |
| 路径 `id` 属于同一租户的其他草稿（学员或课次不同） | 409 `CONFLICT_FEEDBACK_ID_REUSED` |
| 路径 `id` 属于其他租户的记录 | 404，不暴露存在性 |
| `lessonId` 或 `studentId` 不属于当前租户 | 404 |

同一 `(lessonId, studentId)` 只会有一行：并发提交由唯一索引串行化。主键冲突或死锁会重试一次，因此同一 id 的并发重复提交最终合并到同一行，不产生重复数据。

请求体四个可编辑字段为整体替换语义，缺一不可；缺失字段被拒绝，而不是清空已有内容：

```json
{
  "lessonId": "…",
  "studentId": "…",
  "notes": "判别式前半节课听懂，练习跟不上",
  "ratings": { "课堂表现": "专注投入" },
  "knowledgeTags": ["专注投入", "按时完成"],
  "homework": { "done": false }
}
```

| 字段 | 约束 |
| --- | --- |
| `notes` | ≤ 2000 字符（按字符数，不按字节数）；教师原文原样保存、不做 trim；生成流程不得覆盖 |
| `ratings`、`homework` | JSON 对象，各自序列化后 ≤ 4096 字节 |
| `knowledgeTags` | ≤ 50 个；单个标签按去空白后的长度 ≤ 80 字符且不能全为空白；入库保留原字符串 |

响应为反馈本体，含 `id`、`lessonId`、`studentId`、`notes`、`ratings`、`knowledgeTags`、`homework`、`status`、`updatedAt`：

- `status` 取 `draft`、`generating`、`generated`、`failed`、`approved`、`shared`、`revoked`，定义见[数据库设计](database.md)第 3 节。
- `updatedAt` 是服务端写入时间（ISO 8601 UTC），每次成功写入都刷新，合并到已有行时同样刷新；本地草稿与服务端合并且冲突时以它为准。

错误响应：域内校验失败返回 422 `VALIDATION_INVALID`，`errors[0].field` 给出字段名（路径或请求体中的 id 不是 UUID 时为 `id`、`lessonId`、`studentId`）；两个 409 的 `code` 见上表；未认证返回 401（`AUTH_SESSION_EXPIRED` 或 `AUTH_TENANT_REQUIRED`）。

例外：字段缺失或类型不符由请求体反序列化拒绝，返回 422 但响应体不是 Problem Details，客户端此时不能依赖 `code`；统一为 Problem Details 见 #104。标签是否属于内置维度由标签体系负责校验，本接口只校验形状与长度。

## 8. 分享页

`GET /s/{token}` 返回服务端渲染的极简 HTML：

- 无 JavaScript，内联或独立小体积 CSS，目标 2KB 以内。
- 包含反馈正文、图片、课次日期与「AI 辅助生成，教师已审核」标识。
- 响应头：`Cache-Control: no-store`、`X-Robots-Tag: noindex`。
- token 无效、过期或吊销时返回 404 页面，不区分具体原因。

不复用 SPA：家长在移动网络下不需要下载整个前端包。

## 9. 契约生成

```text
utoipa 注解 → openapi.json（提交仓库）→ openapi-typescript → packages/api-types
```

生成：

```bash
bash scripts/generate-openapi.sh
```

规则：

1. 后端是唯一来源，前端禁止手写接口类型。
2. `openapi.json` 与 `packages/api-types/src/schema.d.ts` 必须提交；`bash scripts/check-openapi.sh` 重新生成并比对，漂移即失败。
3. 每个端点必须有 `#[utoipa::path]`、唯一 `operation_id` 与响应 schema；新增端点未注解会被契约任务发现。
4. 表单校验用前端自有 Zod schema，不复用后端类型做运行时校验。

## 10. 版本

- 路径带 `/v1`。
- 破坏性变更升 `/v2`，`/v1` 保留至少 90 天。
- 新增可选字段不算破坏性变更。
