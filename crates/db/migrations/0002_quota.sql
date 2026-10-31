-- 配额与权益表；RLS 策略由业务表 RLS 任务统一添加。

CREATE TABLE usage_counters (
  tenant_id uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  period    date NOT NULL,
  metric    text NOT NULL,
  used      int  NOT NULL DEFAULT 0,
  PRIMARY KEY (tenant_id, period, metric)
);

COMMENT ON TABLE usage_counters IS '租户按自然月与指标累计的用量';
COMMENT ON COLUMN usage_counters.tenant_id IS '所属租户';
COMMENT ON COLUMN usage_counters.period IS '统计周期起始日，按东八区自然月';
COMMENT ON COLUMN usage_counters.metric IS '计数指标，例如 feedback_generated';
COMMENT ON COLUMN usage_counters.used IS '本周期已使用数量';

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

COMMENT ON TABLE entitlements IS '租户增值包与额度增量';
COMMENT ON COLUMN entitlements.id IS '权益记录主键';
COMMENT ON COLUMN entitlements.tenant_id IS '所属租户';
COMMENT ON COLUMN entitlements.addon_code IS '增值包编码';
COMMENT ON COLUMN entitlements.starts_at IS '生效时间';
COMMENT ON COLUMN entitlements.ends_at IS '失效时间；为空表示长期有效';
COMMENT ON COLUMN entitlements.quota_delta IS '相对免费层的额度增量';
COMMENT ON COLUMN entitlements.created_at IS '创建时间';
