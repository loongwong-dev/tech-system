# ADR-0017：反馈草稿写入采用客户端生成 id 的幂等 upsert

- 状态：Proposed
- 日期：2026-11-02
- 决策者：仓库维护者
- 相关：[API 契约](../design/platform/api-contract.md)、[数据库设计](../design/platform/database.md)、[前端架构](../design/platform/frontend.md)、[ADR-0004](0004-tenant-isolation.md)

## 背景

课次录入页在弱网下工作：编辑先落 IndexedDB，防抖 1 秒后把每名学员的完整草稿推给服务端（[前端架构](../design/platform/frontend.md) 第 4 节）。硬性要求：

- 契约只提供 `GET /api/v1/lessons/{id}/feedbacks` 与 `PUT /api/v1/feedbacks/{id}`，没有创建草稿的 `POST` 端点（[API 契约](../design/platform/api-contract.md) 第 2 节）。
- 数据库有 `UNIQUE (lesson_id, student_id)`：一节课一名学员只能有一份反馈（[数据库设计](../design/platform/database.md) 第 3 节）。
- 本地草稿按 `studentId` 组织，不保存反馈 id，离线创建的草稿在首次同步前没有服务端身份。
- 同步是防抖加重试：同一份内容可能在短时间内提交多次，也可能被并发提交。
- 不做什么：不引入 CRDT 或字段级 patch，不做服务端草稿预生成。

## 决策

`PUT /api/v1/feedbacks/{id}` 是租户范围内的幂等 upsert，`id` 由客户端生成：

1. 先校验四个可编辑字段的形状与上限（超限返回 422），再校验 `lessonId` 与 `studentId` 属于当前租户（否则 404；外键不区分租户，缺这一步可以把自己的反馈挂到别家课次上）。
2. 校验路径 id 的归属：指向同一租户的其他草稿（学员或课次不同）返回 409 `CONFLICT_FEEDBACK_ID_REUSED`；这一步先于状态检查。
3. 主键未占用且 `(lesson_id, student_id)` 无记录时插入，初始状态 `draft`。
4. 路径 id 已指向同一份草稿，或 `(lesson_id, student_id)` 已有草稿且该行状态为 `draft` 时，覆盖四个可编辑字段，并返回该行的真实 `id`；客户端必须以后端返回值为准。
5. 命中行状态不是 `draft` 时返回 409 `CONFLICT_INVALID_TRANSITION`；`INSERT ... ON CONFLICT ... DO UPDATE ... WHERE status = 'draft'` 返回 0 行正是这一情况。
6. 路径 id 已存在但当前租户不可见（通常属于其他租户）时返回 404，不暴露存在性。
7. 冲突处理交给单条 `INSERT ... ON CONFLICT (lesson_id, student_id) DO UPDATE`，并发提交由唯一索引串行化，不产生重复行。主键冲突、序列化失败与死锁会中止事务，因此在全新事务里重试一次（最多两次尝试）；同一 id 的并发首次写入因此在重试后合并到已提交的那一行。

行为细节（检查顺序、字段口径、状态码）以 [API 契约](../design/platform/api-contract.md) 第 7.2 节为准，本文只记录取舍。

## 后果

### 正面

- 离线草稿可以先生成本地 id，重试天然安全，不需要先联网再录入。
- 「一节课一名学员一份反馈」由数据库唯一约束兜底，不依赖前端正确性。
- 不新增端点，前端契约与既有 `PUT` 路径保持一致。

### 负面 / 代价

- 响应 `id` 可能与请求 id 不同（合并到已有行时），客户端必须处理这个映射。
- 客户端 id 由前端生成，服务端无法在创建前校验其唯一性。
- 四个字段是整体替换，防抖提交必须携带完整状态；漏字段是 422 而不是部分更新。

### 风险与缓解

- 风险：客户端忽略返回 id，本地出现两条指向同一学员的草稿。
  缓解：本地草稿按 `studentId` 归并，不按反馈 id 建索引（[前端架构](../design/platform/frontend.md) 第 4 节）。
- 风险：用 `PUT` 承担创建职责，与常见 REST 语义不一致。
  缓解：在契约中写明 upsert 语义与各状态码，并用并发测试固定行为。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 新增 `POST /api/v1/lessons/{id}/feedbacks` 创建，`PUT` 只更新 | 严格 REST 语义 | 契约多一个端点；创建失败后的重试仍可能重复提交 | 第 6 节的 `Idempotency-Key` 只按请求键去重且窗口 24 小时，离线草稿跨越窗口重放时会撞 `UNIQUE (lesson_id, student_id)`，收益仅是语义整齐 |
| 服务端生成 id，客户端先提交再回填 | 没有客户端 id 复用风险 | 离线场景必须先联网；防抖重试仍需额外幂等机制 | 与离线优先目标冲突 |
| `(lesson_id, student_id)` 冲突时返回 409 | 语义最严格 | 并发防抖会稳定失败，调用方必须重取再写 | 把并发复杂度推给每个调用方 |
| `GET` 时按班级学员自动预生成草稿行 | 前端总能拿到 id | 读接口产生写副作用；空课次返回非空数组，与验收标准冲突 | 读接口不应带副作用 |

## 验证方式

- 命令或检查项：`cargo test -p api --test feedbacks`，覆盖幂等重试覆盖正文、合并到已有行并覆盖四个字段、非 `draft` 拒绝、id 复用、跨租户 id 保持 404 与并发写入单行。
- 观察指标：同一 `(lesson_id, student_id)` 恒为 1 行；同一批并发写入的响应 `id` 全部相同，且最终正文来自最后一次写入。
- 复核时间：M1 录入退出条件达成后一次；#33、#34 落地后确认前端按后端返回 id 归并。

## 参考

- PostgreSQL `INSERT ... ON CONFLICT`：<https://www.postgresql.org/docs/17/sql-insert.html>（访问日期：2026-11-02）
- RFC 9110 `PUT` 语义：<https://www.rfc-editor.org/rfc/rfc9110#section-9.3.4>（访问日期：2026-11-02）
