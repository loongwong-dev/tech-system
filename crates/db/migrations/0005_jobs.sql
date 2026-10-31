-- 后台任务队列：单表 + SKIP LOCKED 领取，租约到期由 worker 回收。
-- 设计决策见 docs/adr/0002-postgres-job-queue.md。

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
  -- 只有 running 状态持有租约；成功、死信与取消都必须释放锁。
  CHECK ((status = 'running') = (locked_by IS NOT NULL)),
  CHECK ((status = 'running') = (locked_until IS NOT NULL)),
  -- 终态必须有结束时间，非终态不得有结束时间。
  CHECK ((status IN ('succeeded', 'dead', 'cancelled')) = (finished_at IS NOT NULL))
);

COMMENT ON TABLE jobs IS '后台任务；事务内入队，worker 通过 SKIP LOCKED 领取';
COMMENT ON COLUMN jobs.id IS '任务主键';
COMMENT ON COLUMN jobs.tenant_id IS '所属租户；worker 执行时必须重新开启租户事务';
COMMENT ON COLUMN jobs.kind IS '任务类型，例如 feedback.generate_batch';
COMMENT ON COLUMN jobs.dedupe_key IS '幂等键；仅在 pending/running 状态下参与唯一约束';
COMMENT ON COLUMN jobs.payload IS '任务输入 JSON；不存放密码、令牌等敏感信息';
COMMENT ON COLUMN jobs.status IS '状态：pending、running、succeeded、dead、cancelled';
COMMENT ON COLUMN jobs.attempts IS '已领取次数；每次领取加一';
COMMENT ON COLUMN jobs.max_attempts IS '最大领取次数；达到后失败进入 dead';
COMMENT ON COLUMN jobs.run_at IS '最早可领取时间；失败退避会推迟';
COMMENT ON COLUMN jobs.locked_by IS '当前持有租约的 worker 标识';
COMMENT ON COLUMN jobs.locked_until IS '租约到期时间；到期后由回收循环重新入队';
COMMENT ON COLUMN jobs.last_error IS '最近一次失败原因；用于排障与死信审计';
COMMENT ON COLUMN jobs.created_at IS '入队时间';
COMMENT ON COLUMN jobs.updated_at IS '状态或租约最后变更时间';
COMMENT ON COLUMN jobs.finished_at IS '终态时间；pending/running 为空';

-- 幂等只约束活跃任务：完成后的同一个 dedupe_key 可以再次入队。
CREATE UNIQUE INDEX jobs_active_dedupe_idx ON jobs (tenant_id, kind, dedupe_key)
  WHERE dedupe_key IS NOT NULL AND status IN ('pending', 'running');

-- 领取按最早可运行时间排序；部分索引避免扫描终态历史。
CREATE INDEX jobs_pending_idx ON jobs (run_at, created_at)
  WHERE status = 'pending';
-- 回收只扫描持有过期租约的运行中任务。
CREATE INDEX jobs_expired_idx ON jobs (locked_until)
  WHERE status = 'running';
CREATE INDEX jobs_tenant_created_idx ON jobs (tenant_id, created_at DESC);

ALTER TABLE jobs ENABLE ROW LEVEL SECURITY;
ALTER TABLE jobs FORCE ROW LEVEL SECURITY;
CREATE POLICY jobs_tenant ON jobs
  USING (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid)
  WITH CHECK (tenant_id = NULLIF(current_setting('app.tenant_id', true), '')::uuid);
