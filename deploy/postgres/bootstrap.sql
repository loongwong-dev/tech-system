-- PostgreSQL 角色引导：为应用创建两个最小权限角色。
--
-- 运行者：数据库表所有者或 DBA，需要 CREATEROLE 与对象授权权限。
-- 运行时机：首次部署、或新增数据库/授权时；脚本可重复执行。
-- 运行方式：psql "$DATABASE_URL" -f deploy/postgres/bootstrap.sql
--
-- 本脚本只负责角色与权限，不创建业务表；表结构由 crates/db/migrations/ 下的迁移创建。
-- 密码不写入仓库：角色以 LOGIN 创建但初始无密码，部署时由密钥管理或
-- `ALTER ROLE ... PASSWORD` 单独设置；若使用 IAM/证书认证则无需密码。
--
-- 两个应用角色：
--   app_tenant  受 RLS 约束，供租户池使用；
--   app_admin   具备 BYPASSRLS，供管理池使用。
-- 两者都没有 DDL 权限，不能创建、修改或删除表。

-- 角色是集群级对象；已存在时跳过，保证脚本可重复执行。
-- \gexec 是 psql 元命令：先查询再执行生成的 SQL，避免在 DO 块里拼接动态 DDL。
SELECT 'CREATE ROLE app_tenant LOGIN'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'app_tenant')\gexec

SELECT 'CREATE ROLE app_admin LOGIN BYPASSRLS'
WHERE NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'app_admin')\gexec

-- 显式重置关键属性，避免角色被手工改动后偏离预期。
ALTER ROLE app_tenant NOBYPASSRLS;
ALTER ROLE app_admin BYPASSRLS;

-- schema 使用权：没有 USAGE 时无法解析表名。
GRANT USAGE ON SCHEMA public TO app_tenant, app_admin;

-- 已有表的 DML 权限。表由表所有者创建，应用角色不需要也不允许 DDL。
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_tenant, app_admin;

-- 序列权限用于 bigserial 等自增主键；当前平台表使用 UUID，后续审计表会用到。
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_tenant, app_admin;

-- 未来由“当前执行本脚本的表所有者”创建的表与序列自动继承权限。
-- 若迁移改由其他角色执行，需要在该角色上重新执行这两条默认权限。
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_tenant, app_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_tenant, app_admin;

-- 撤销 schema 级 CREATE：角色仍可读写已授权表，但不能执行 DDL。
REVOKE CREATE ON SCHEMA public FROM app_tenant, app_admin;

-- 默认 PUBLIC 已具备 CONNECT；若生产环境撤销了 PUBLIC CONNECT，
-- 需要由运维额外执行：GRANT CONNECT ON DATABASE <db> TO app_tenant, app_admin;
