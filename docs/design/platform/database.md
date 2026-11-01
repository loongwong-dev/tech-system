# 数据库设计

全系统库表结构、索引、行级安全策略与数据生命周期。作者与读者是维护者本人。

- **类型**：项目设计
- **覆盖范围**：业务表定义、索引、RLS 策略、删除策略、配额扣减、迁移规则。
- **不覆盖范围**：查询实现与仓储层代码；队列行为的取舍见 [ADR-0002](../../adr/0002-postgres-job-queue.md)。
- **状态**：已定，随迁移脚本同步更新。

## 1. 命名约定

| 约定 | 规则 |
| --- | --- |
| 表名 | 复数 `snake_case` |
| 时间 | 一律 `timestamptz`；纯日期用 `date` |
| 主键 | `uuid`，`gen_random_uuid()` |
| 租户列 | 业务表必须有 `tenant_id`，并参与复合索引 |
| 布尔 | `is_` 前缀或状态列，不用 `tinyint` |

**课次必须叫 `lessons`，不能叫 `session`。** 认证会话占用 `auth_sessions`，两者混用会让 `session_id` 出现两种含义。

## 2. 平台表

```sql
CREATE TABLE tenants (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name       text NOT NULL,
  slug       text NOT NULL UNIQUE,
  status     text NOT NULL DEFAULT 'active'
             CHECK (status IN ('active', 'frozen', 'deleted')),
  created_at timestamptz NOT NULL DEFAULT now(),
  deleted_at timestamptz
);

CREATE TABLE users (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email         text NOT NULL,
  phone         text,
  password_hash text NOT NULL,
  status        text NOT NULL DEFAULT 'active',
  created_at    timestamptz NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX users_email_lower_idx ON users (lower(email));

CREATE TABLE memberships (
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role       text NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'member')),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, user_id)
);

CREATE TABLE auth_sessions (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tenant_id  uuid REFERENCES tenants(id) ON DELETE CASCADE,
  token_hash bytea NOT NULL UNIQUE,
  expires_at timestamptz NOT NULL,
  revoked_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX auth_sessions_user_idx ON auth_sessions (user_id);

CREATE TABLE platform_admins (
  user_id      uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  role         text NOT NULL CHECK (role IN ('super_admin', 'ops', 'finance')),
  totp_secret  bytea,
  totp_enabled boolean NOT NULL DEFAULT false,
  created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE invite_codes (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  code_hash  bytea NOT NULL UNIQUE,
  note       text,
  max_uses   int NOT NULL DEFAULT 1,
  used_count int NOT NULL DEFAULT 0,
  expires_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now(),
  CHECK (used_count <= max_uses)
);
```

`totp_secret` 存 AEAD 密文（密钥来自环境变量），不是明文共享密钥。

## 3. 业务表

```sql
CREATE TABLE students (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name        text NOT NULL,
  grade       text,
  subject     text,
  archived_at timestamptz,
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX students_tenant_idx ON students (tenant_id, archived_at);

CREATE TABLE classes (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name       text NOT NULL,
  subject    text,
  archived_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX classes_tenant_idx ON classes (tenant_id);
CREATE INDEX classes_tenant_archived_idx ON classes (tenant_id, archived_at);

CREATE TABLE lessons (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  class_id    uuid NOT NULL REFERENCES classes(id) ON DELETE CASCADE,
  lesson_date date NOT NULL,
  topic       text,
  status      text NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'done')),
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX lessons_tenant_date_idx ON lessons (tenant_id, lesson_date DESC);

CREATE TABLE feedbacks (
  id             uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id      uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  lesson_id      uuid NOT NULL REFERENCES lessons(id) ON DELETE CASCADE,
  student_id     uuid NOT NULL REFERENCES students(id) ON DELETE CASCADE,
  notes          text NOT NULL DEFAULT '',
  ratings        jsonb NOT NULL DEFAULT '{}'::jsonb,
  knowledge_tags text[] NOT NULL DEFAULT '{}',
  homework       jsonb NOT NULL DEFAULT '{}'::jsonb,
  rendered       jsonb,
  status         text NOT NULL DEFAULT 'draft'
                 CHECK (status IN ('draft', 'generating', 'generated',
                                   'failed', 'approved', 'shared', 'revoked')),
  generated_by   text CHECK (generated_by IN ('ai', 'template')),
  regen_count    smallint NOT NULL DEFAULT 0 CHECK (regen_count BETWEEN 0 AND 3),
  prompt_version text,
  created_at     timestamptz NOT NULL DEFAULT now(),
  updated_at     timestamptz NOT NULL DEFAULT now(),
  UNIQUE (lesson_id, student_id)
);
CREATE INDEX feedbacks_tenant_status_idx ON feedbacks (tenant_id, status);

CREATE TABLE share_artifacts (
  id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id       uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  feedback_id     uuid NOT NULL REFERENCES feedbacks(id) ON DELETE CASCADE,
  kind            text NOT NULL CHECK (kind IN ('image', 'link')),
  token_hash      bytea UNIQUE,
  expires_at      timestamptz,
  revoked_at      timestamptz,
  first_viewed_at timestamptz,
  view_count      int NOT NULL DEFAULT 0,
  created_at      timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX share_tenant_idx ON share_artifacts (tenant_id, feedback_id);

CREATE TABLE usage_counters (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  period    date NOT NULL,
  metric    text NOT NULL,
  used      int  NOT NULL DEFAULT 0,
  PRIMARY KEY (tenant_id, period, metric)
);

CREATE TABLE entitlements (
  id          uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id   uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  addon_code  text NOT NULL,
  starts_at   timestamptz NOT NULL DEFAULT now(),
  ends_at     timestamptz,
  quota_delta int NOT NULL DEFAULT 0,
  created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX entitlements_tenant_idx ON entitlements (tenant_id, ends_at);

CREATE TABLE feedback_templates (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id  uuid REFERENCES tenants(id) ON DELETE CASCADE,
  name       text NOT NULL,
  tag_preset jsonb NOT NULL DEFAULT '{}'::jsonb,
  body       text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX feedback_templates_tenant_idx ON feedback_templates (tenant_id);

CREATE TABLE audit_logs (
  id         bigserial PRIMARY KEY,
  tenant_id  uuid REFERENCES tenants(id) ON DELETE SET NULL,
  actor_id   uuid REFERENCES users(id) ON DELETE SET NULL,
  action     text NOT NULL,
  target     text,
  before     jsonb,
  after      jsonb,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX audit_tenant_time_idx ON audit_logs (tenant_id, created_at DESC);

CREATE TABLE jobs (
  id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id    uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  kind         text NOT NULL,
  dedupe_key   text,
  payload      jsonb NOT NULL DEFAULT '{}'::jsonb,
  status       text NOT NULL DEFAULT 'pending'
               CHECK (status IN ('pending', 'running', 'succeeded', 'dead', 'cancelled')),
  attempts     int NOT NULL DEFAULT 0 CHECK (attempts >= 0),
  max_attempts int NOT NULL DEFAULT 5 CHECK (max_attempts > 0),
  run_at       timestamptz NOT NULL DEFAULT now(),
  locked_by    text,
  locked_until timestamptz,
  last_error   text,
  created_at   timestamptz NOT NULL DEFAULT now(),
  updated_at   timestamptz NOT NULL DEFAULT now(),
  finished_at  timestamptz,
  CHECK ((status = 'running') = (locked_by IS NOT NULL)),
  CHECK ((status = 'running') = (locked_until IS NOT NULL)),
  CHECK ((status IN ('succeeded', 'dead', 'cancelled')) = (finished_at IS NOT NULL))
);
CREATE UNIQUE INDEX jobs_active_dedupe_idx ON jobs (tenant_id, kind, dedupe_key)
  WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'running');
CREATE INDEX jobs_pending_idx ON jobs (run_at, created_at)
  WHERE status = 'pending';
CREATE INDEX jobs_expired_idx ON jobs (locked_until)
  WHERE status = 'running';
CREATE INDEX jobs_tenant_created_idx ON jobs (tenant_id, created_at DESC);
```

任务表既是业务表也是队列；设计取舍见 [ADR-0002](../../adr/0002-postgres-job-queue.md)。`dead` 是死信终态，`last_error` 保留最后一次失败原因。

## 4. 索引清单

| 索引 | 服务的查询 |
| --- | --- |
| `feedbacks (lesson_id, student_id)` 唯一 | 一节课一份反馈，天然去重 |
| `feedbacks (tenant_id, status)` | 课次页批量读取、待审核列表 |
| `lessons (tenant_id, lesson_date DESC)` | 首页课次列表 |
| `students (tenant_id, archived_at)` | 学员列表，过滤已归档 |
| `share_artifacts (token_hash)` 唯一 | 家长打开链接 |
| `usage_counters` 主键 | 配额原子扣减 |
| `audit_logs (tenant_id, created_at DESC)` | 管理端审计筛选 |
| `jobs_active_dedupe_idx` 部分唯一 | 同一租户、类型与幂等键只保留一条活跃任务 |
| `jobs_pending_idx` 部分索引 | worker 按 `run_at` 领取待执行任务 |
| `jobs_expired_idx` 部分索引 | 回收租约过期的运行中任务 |

`WHERE tenant_id = $1` 必须保留，即使已启用 RLS：索引依赖这个等值条件，RLS 只负责兜底安全。

## 5. 行级安全

```sql
ALTER TABLE feedbacks ENABLE ROW LEVEL SECURITY;
ALTER TABLE feedbacks FORCE ROW LEVEL SECURITY;

CREATE POLICY feedbacks_tenant ON feedbacks
  USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
  WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
```

三条规则：

1. `FORCE ROW LEVEL SECURITY` 不能省：默认表所有者绕过策略，加 `FORCE` 后才对所有者生效。
2. 策略条件用 `NULLIF(current_setting('app.tenant_id', true), '')::uuid`：缺失时返回 NULL，`SET LOCAL` 结束后返回空串也不会触发 UUID 转换错误；查询得到空集而不是全表。
3. 策略必须同时写 `USING` 与 `WITH CHECK`，否则可以插入到别的租户。

带平台内置数据的表（如 `feedback_templates`）需要放宽读取、收紧写入：

```sql
CREATE POLICY feedback_templates_read ON feedback_templates FOR SELECT
  USING (tenant_id IS NULL OR tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);

CREATE POLICY feedback_templates_write ON feedback_templates FOR INSERT
  WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
```

租户能读到平台内置模板，但只能创建自己租户的模板。平台内置模板由管理池写入。

### 例外清单

以下表**不启用** RLS，只能通过管理池访问：

- `users`
- `tenants`
- `memberships`（租户角色需要由管理池读取，不授予租户池权限）
- `invite_codes`
- `platform_admins`
- `auth_sessions`（登录与登出需要跨租户查找，由管理池处理）

`audit_logs` 启用 RLS，平台级记录 `tenant_id` 为空，租户池天然看不到。

`jobs` 与其他业务表一样启用 RLS；worker 使用管理池跨租户领取，handler 写业务数据时必须按任务的 `tenant_id` 重新开启租户事务。

新增表时必须回答两个问题：是否属于例外清单；若不属于，策略是否已写。

## 6. 删除策略

学员删除为**级联删除**：

```text
students → feedbacks → share_artifacts
```

`share_artifacts` 随反馈删除后，已发出的家长链接立即失效。

配套要求：

1. 删除前要求输入学员姓名确认，并记录删除的反馈数量到审计。
2. 删除使用单条 `DELETE FROM students WHERE id = $1 AND tenant_id = $2`，级联由外键完成。
3. 反馈内容若已被渲染成图片文件，需同时删除磁盘文件。

`tenants` 与 `users` 使用 `status` 与 `deleted_at` 软删除；业务数据随租户删除时物理级联。

## 7. 配额扣减

采用「预留 + 失败退还」：

```sql
UPDATE usage_counters
SET used = used + 1
WHERE tenant_id = $1 AND period = $2 AND metric = $3
  AND used < $4
RETURNING used;
```

返回零行即配额不足。任务永久失败时执行反向更新退还，必须在同一个任务状态变更中完成，避免重复退还。

## 8. 迁移规则

- 迁移文件存放在 `crates/db/migrations/`，按序号命名，不修改已应用的文件。
- 从仓库根执行 `sqlx migrate run --source crates/db/migrations`，或在 `crates/db/` 下直接执行 `sqlx migrate run`。
- 迁移由表所有者角色执行，与应用角色分离；角色引导见 `deploy/postgres/bootstrap.sql`，必须在迁移之后执行。
- 应用角色 `app_tenant` 与 `app_admin` 均无 DDL 权限；`app_tenant` 也不能访问例外表与 `_sqlx_migrations`。连接串与池上限见[项目配置说明](../../configuration.md)。
- 新增业务表必须同时提交 RLS 策略与策略覆盖测试。
- 破坏性变更（删列、改类型）必须分两步：先发布兼容读写，再移除旧结构。

## 9. 验证

- `sqlx migrate run --source crates/db/migrations` 在空库上可完整重放。
- 策略覆盖测试遍历业务表，断言每张表都启用 RLS 且至少有一条策略。
- 跨租户查询测试断言返回空集。
