-- no-transaction
-- 课次列表按 (tenant_id, lesson_date DESC, id DESC) 做 keyset 分页。
-- 并发构建，避免阻塞课次表读写；失败重试前先清理可能残留的无效索引。
CREATE INDEX CONCURRENTLY lessons_tenant_date_id_idx
  ON lessons (tenant_id, lesson_date DESC, id DESC);
