-- PostgreSQL 角色引导：为应用创建两个最小权限角色。
--
-- 执行顺序：先执行数据库迁移，再执行本脚本。
-- 迁移负责创建平台表与 `_sqlx_migrations`；本脚本据此授权，并撤销
-- 租户角色对平台例外表、以及两个应用角色对迁移历史的权限。
--
-- 运行者：数据库表所有者或 DBA，需要 CREATEROLE 与对象授权权限。
-- 运行时机：首次部署、或每次新增表/调整权限后；脚本可重复执行。
-- 运行方式：psql "$DATABASE_URL" -f deploy/postgres/bootstrap.sql
--
-- 本脚本只负责角色与权限，不创建业务表；表结构由 crates/db/migrations/ 下的迁移创建。
-- 密码不写入仓库：角色以 LOGIN 创建但初始无密码，部署时由密钥管理或
-- `ALTER ROLE ... PASSWORD` 单独设置；若使用 IAM/证书认证则无需密码。
--
-- 两个应用角色：
--   app_tenant  受 RLS 约束，只能访问业务表；
--   app_admin   具备 BYPASSRLS，用于跨租户的管理操作。
-- 两者都没有 DDL 权限，不能创建、修改或删除表。

-- 迁移必须先执行：下面的授权与撤销依赖平台表和 _sqlx_migrations。
DO $$
BEGIN
  IF to_regclass('public.users') IS NULL
     OR to_regclass('public._sqlx_migrations') IS NULL THEN
    RAISE EXCEPTION '请先执行 sqlx migrate run，再运行 bootstrap.sql';
  END IF;
END
$$;

-- 角色是集群级对象；已存在时跳过，保证脚本可重复执行。
-- 使用异常处理而不是 psql 的 \gexec，使整个文件是纯 SQL，
-- 可以在集成测试中通过 sqlx 直接执行。
DO $$
BEGIN
  BEGIN
    EXECUTE 'CREATE ROLE app_tenant LOGIN';
  EXCEPTION WHEN duplicate_object OR unique_violation THEN
    NULL;
  END;
  BEGIN
    EXECUTE 'CREATE ROLE app_admin LOGIN BYPASSRLS';
  EXCEPTION WHEN duplicate_object OR unique_violation THEN
    NULL;
  END;
END
$$;

-- 显式重置关键属性，避免角色被手工改动后偏离预期。
ALTER ROLE app_tenant NOBYPASSRLS;
ALTER ROLE app_admin BYPASSRLS;

-- schema 使用权：没有 USAGE 时无法解析表名。
GRANT USAGE ON SCHEMA public TO app_tenant, app_admin;

-- 先授予现有表与序列的 DML；平台例外表在下面显式撤销。
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO app_tenant, app_admin;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO app_tenant, app_admin;

-- 未来由“当前执行本脚本的表所有者”创建的表与序列自动继承 DML 权限。
-- 本脚本必须与迁移使用同一个数据库所有者角色执行；若迁移改由其他角色执行，
-- 需要在该角色上重新执行这两条默认权限。
-- 新增平台例外表时，必须把表名加入下面的 REVOKE 清单并重跑本脚本。
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO app_tenant, app_admin;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO app_tenant, app_admin;

-- 平台例外表不启用 RLS，只能通过管理池访问；租户角色不得有任何权限。
REVOKE ALL ON TABLE tenants, users, memberships, invite_codes, platform_admins, auth_sessions
  FROM app_tenant;

-- 迁移历史只由表所有者维护；应用角色不可读写。
REVOKE ALL ON TABLE _sqlx_migrations FROM app_tenant, app_admin;

-- 撤销 schema 级 CREATE，并清理 PostgreSQL 15 之前 PUBLIC 的默认 CREATE。
REVOKE CREATE ON SCHEMA public FROM app_tenant, app_admin;
REVOKE CREATE ON SCHEMA public FROM PUBLIC;

-- 默认 PUBLIC 已具备 CONNECT；若生产环境撤销了 PUBLIC CONNECT，
-- 需要由运维额外执行：GRANT CONNECT ON DATABASE <db> TO app_tenant, app_admin;
