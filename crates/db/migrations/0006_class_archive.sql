-- 班级归档：归档后不再出现在默认列表，但仍保留课次与历史数据。

ALTER TABLE classes ADD COLUMN archived_at timestamptz;

CREATE INDEX classes_tenant_archived_idx ON classes (tenant_id, archived_at);
