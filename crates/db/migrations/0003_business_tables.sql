-- 业务表：学员、班级、课次、反馈、分享物、模板与审计。
-- RLS 策略在 0004_rls_policies.sql 中启用。

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
COMMENT ON TABLE students IS '学员；仅存必要字段，禁止证件号与家长联系方式';

CREATE TABLE classes (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  name       text NOT NULL,
  subject    text,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX classes_tenant_idx ON classes (tenant_id);
COMMENT ON TABLE classes IS '班级；属于租户';

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
COMMENT ON TABLE lessons IS '课次；录入的组织单位，不叫 session';

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
COMMENT ON TABLE feedbacks IS '反馈；notes 为教师原文，生成不得覆盖';

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
COMMENT ON TABLE share_artifacts IS '分享物；链接只存 token 哈希';

CREATE TABLE feedback_templates (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  tenant_id  uuid REFERENCES tenants(id) ON DELETE CASCADE,
  name       text NOT NULL,
  tag_preset jsonb NOT NULL DEFAULT '{}'::jsonb,
  body       text NOT NULL,
  created_at timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX feedback_templates_tenant_idx ON feedback_templates (tenant_id);
COMMENT ON TABLE feedback_templates IS '反馈模板；tenant_id 为空表示平台内置';

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
COMMENT ON TABLE audit_logs IS '审计日志；只追加，before/after 为 JSON 快照';
