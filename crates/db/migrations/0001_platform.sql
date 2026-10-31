-- 平台基础表：租户、账号、会话、管理员与邀请码。
-- 业务表随后续迁移创建；本文件不启用 RLS，平台表只允许管理池访问。

CREATE TABLE tenants (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  name       text NOT NULL,
  slug       text NOT NULL UNIQUE,
  status     text NOT NULL DEFAULT 'active'
             CHECK (status IN ('active', 'frozen', 'deleted')),
  created_at timestamptz NOT NULL DEFAULT now(),
  deleted_at timestamptz
);

COMMENT ON TABLE tenants IS '租户：一个教师或机构对应一个租户';
COMMENT ON COLUMN tenants.id IS '租户主键';
COMMENT ON COLUMN tenants.name IS '租户显示名称';
COMMENT ON COLUMN tenants.slug IS '租户唯一短标识';
COMMENT ON COLUMN tenants.status IS '租户状态：active、frozen、deleted';
COMMENT ON COLUMN tenants.created_at IS '创建时间';
COMMENT ON COLUMN tenants.deleted_at IS '软删除时间；为空表示未删除';

CREATE TABLE users (
  id            uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  email         text NOT NULL,
  phone         text,
  password_hash text NOT NULL,
  status        text NOT NULL DEFAULT 'active',
  created_at    timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE users IS '平台用户账号；家长不是平台账号主体，不在此表';
COMMENT ON COLUMN users.id IS '用户主键';
COMMENT ON COLUMN users.email IS '登录邮箱；唯一性按小写比较';
COMMENT ON COLUMN users.phone IS '手机号，可选';
COMMENT ON COLUMN users.password_hash IS 'Argon2id 密码哈希';
COMMENT ON COLUMN users.status IS '账号状态';
COMMENT ON COLUMN users.created_at IS '创建时间';

CREATE UNIQUE INDEX users_email_lower_idx ON users (lower(email));

CREATE TABLE memberships (
  tenant_id  uuid NOT NULL REFERENCES tenants(id) ON DELETE CASCADE,
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  role       text NOT NULL DEFAULT 'member' CHECK (role IN ('owner', 'member')),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, user_id)
);

COMMENT ON TABLE memberships IS '用户与租户的成员关系及角色';
COMMENT ON COLUMN memberships.tenant_id IS '所属租户';
COMMENT ON COLUMN memberships.user_id IS '所属用户';
COMMENT ON COLUMN memberships.role IS '租户内角色：owner、member';
COMMENT ON COLUMN memberships.created_at IS '加入时间';

CREATE TABLE auth_sessions (
  id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id    uuid NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  tenant_id  uuid REFERENCES tenants(id) ON DELETE CASCADE,
  token_hash bytea NOT NULL UNIQUE,
  expires_at timestamptz NOT NULL,
  revoked_at timestamptz,
  created_at timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE auth_sessions IS '登录会话；只存 token 哈希，不存明文';
COMMENT ON COLUMN auth_sessions.id IS '会话主键';
COMMENT ON COLUMN auth_sessions.user_id IS '所属用户';
COMMENT ON COLUMN auth_sessions.tenant_id IS '当前租户上下文；为空表示尚未选择租户';
COMMENT ON COLUMN auth_sessions.token_hash IS '会话 token 的 SHA-256 哈希';
COMMENT ON COLUMN auth_sessions.expires_at IS '过期时间';
COMMENT ON COLUMN auth_sessions.revoked_at IS '吊销时间；为空表示有效';
COMMENT ON COLUMN auth_sessions.created_at IS '创建时间';

CREATE INDEX auth_sessions_user_idx ON auth_sessions (user_id);

CREATE TABLE platform_admins (
  user_id      uuid PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
  role         text NOT NULL CHECK (role IN ('super_admin', 'ops', 'finance')),
  totp_secret  bytea,
  totp_enabled boolean NOT NULL DEFAULT false,
  created_at   timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE platform_admins IS '平台管理员角色与双因素配置';
COMMENT ON COLUMN platform_admins.user_id IS '管理员用户';
COMMENT ON COLUMN platform_admins.role IS '平台角色：super_admin、ops、finance';
COMMENT ON COLUMN platform_admins.totp_secret IS 'TOTP 密钥的 AEAD 密文';
COMMENT ON COLUMN platform_admins.totp_enabled IS '是否已启用 TOTP';
COMMENT ON COLUMN platform_admins.created_at IS '创建时间';

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

COMMENT ON TABLE invite_codes IS '注册邀请码；只存哈希与使用计数';
COMMENT ON COLUMN invite_codes.id IS '邀请码主键';
COMMENT ON COLUMN invite_codes.code_hash IS '邀请码的 SHA-256 哈希';
COMMENT ON COLUMN invite_codes.note IS '运营备注';
COMMENT ON COLUMN invite_codes.max_uses IS '最大可用次数';
COMMENT ON COLUMN invite_codes.used_count IS '已使用次数';
COMMENT ON COLUMN invite_codes.expires_at IS '过期时间；为空表示不过期';
COMMENT ON COLUMN invite_codes.created_at IS '创建时间';
