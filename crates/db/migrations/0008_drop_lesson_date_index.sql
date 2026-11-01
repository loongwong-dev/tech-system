-- no-transaction
-- 新索引覆盖旧索引的列前缀；并发删除旧索引，避免阻塞课次表读写。
DROP INDEX CONCURRENTLY IF EXISTS lessons_tenant_date_idx;
