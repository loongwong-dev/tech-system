# 认证与安全

账号、会话、密码、限流与分享链接的安全设计。实现时以本文为准。

- **覆盖范围**：注册、登录、会话、CSRF、限流、租户切换、管理端双因素、分享链接、安全头、审计。
- **不覆盖范围**：接口清单见 [API 契约](api-contract.md)，表结构见 [数据库设计](database.md)。
- **状态**：已定，安全相关变更必须同步更新。

## 1. 会话

| 项 | C 端 | 管理端 |
| --- | --- | --- |
| Cookie 名 | `sid` | `asid` |
| 值 | 32 字节随机 token，base64url | 同上 |
| 数据库存储 | 只存 SHA-256 哈希 | 同上 |
| 属性 | `HttpOnly; Secure; SameSite=Lax; Path=/` | `HttpOnly; Secure; SameSite=Lax; Path=/admin` |
| 有效期 | 30 天滑动 | 12 小时，不滑动 |
| 登录时 | 必须轮换 token | 必须轮换 token |

规则：

1. 只存哈希，数据库泄露无法反推会话。
2. 登录、租户切换、权限变更后轮换 token，防止会话固定。
3. 滑动续期最多每小时写一次 `expires_at`，避免每请求写库。
4. 登出即置 `revoked_at`，不是删除 Cookie 了事。
5. 两个 Cookie 互不通用，服务端按路由分组决定读取哪一个。

## 2. 注册

```text
邮箱 → 密码 → 邀请码 → 原子消耗一次 → 创建 user + tenant + membership(owner) → 建立会话
```

邀请码原子消耗：

```sql
UPDATE invite_codes
SET used_count = used_count + 1
WHERE code_hash = $1 AND used_count < max_uses
  AND (expires_at IS NULL OR expires_at > now())
RETURNING id;
```

返回零行即无效、已用尽或已过期。禁止先查后改。

邀请码为 Crockford Base32 的 10 字符，存哈希，默认单次使用。

## 3. 密码

| 项 | 规则 |
| --- | --- |
| 算法 | Argon2id |
| 参数 | m=19MiB、t=2、p=1（OWASP 推荐） |
| 长度 | ≥ 10，上限 128 |
| 复杂度 | 不强制大小写与符号，改用弱密码黑名单 |
| 用户不存在 | 仍执行一次哈希校验，拉平响应时间 |
| 错误信息 | 登录失败统一文案，不区分邮箱不存在与密码错误 |

**v1 不提供自助密码重置。** 忘记密码由运营在管理端重置并发放临时密码，首次登录强制修改。理由：避免引入 SMTP 链路与邮件模板，也减少一条可被滥用的攻击面。

## 4. CSRF

Cookie 认证加 JSON API，不需要 CSRF token。三层防护：

| 层 | 规则 |
| --- | --- |
| SameSite=Lax | 跨站 `POST`/`PUT`/`PATCH`/`DELETE` 不携带 Cookie |
| Origin 校验 | 所有变更请求要求 `Origin` 或 `Referer` 与本站匹配，缺失或不匹配返回 403 |
| Content-Type | 变更接口只接受 `application/json`，阻断简单表单跨站提交 |

第二层必须实现。只依赖 SameSite 会忽略同站子域与旧浏览器差异。

## 5. 限流与锁定

| 对象 | 策略 | 存储 |
| --- | --- | --- |
| 登录（按 IP） | 10 次/分钟 | 进程内令牌桶 |
| 登录（按账号） | 连续 5 次失败锁 15 分钟 | 数据库 |
| 注册 | 5 次/小时/IP | 进程内 |
| 分享页查看 | 60 次/分钟/IP | 进程内 |
| AI 生成 | 10 次/分钟/租户 | 进程内，配额之外的兜底 |

按账号的锁定必须落库：进程重启即清零的锁等于没有锁。

## 6. 租户切换

```text
POST /auth/tenant { tenantId }
→ 校验 membership → 更新会话 tenant_id → 轮换 token
```

轮换不可省略：旧 token 若仍指向原租户，权限上下文就挂在一个可被重放的字符串上。

## 7. 管理端双因素

平台管理员启用 TOTP：

- 绑定流程生成密钥与二维码，用户输入一次验证码后启用。
- 登录时密码正确后再校验 TOTP，未启用则强制绑定。
- 密钥以 AEAD 密文存库，密钥来自环境变量，不写进仓库。
- 备份码一次性使用，存哈希。

管理员账号能看到全部租户的学员与反馈，一次失窃的影响面是整个平台，这道防线不能省。

## 8. 分享链接

| 项 | 设计 |
| --- | --- |
| 熵 | 32 字节随机，枚举不可行 |
| 存储 | 只存哈希 |
| 有效期 | 7 天 |
| 吊销 | 置 `revoked_at`，立即 404 |
| 已读 | 记录首次查看时间与次数，不记 IP |
| 响应头 | `Cache-Control: no-store`、`X-Robots-Tag: noindex` |

无效、过期、吊销一律返回同一个 404 页面，不区分原因。

## 9. 安全响应头

在 Caddy 层统一添加：

```text
Strict-Transport-Security: max-age=31536000; includeSubDomains
X-Content-Type-Options: nosniff
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
Content-Security-Policy: default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; object-src 'none'; frame-ancestors 'none'
```

分享页样式放独立文件，保持 CSP 严格，不使用 `unsafe-inline`。

## 10. 审计

记录以下事件：登录成功与失败、会话吊销、租户切换、邀请码创建与消耗、管理员开通权益、学员删除、管理端双因素变更。

- 保留 180 天。
- 登录失败记录 IP 哈希，不存明文，30 天后清理。
- 审计只追加，不提供删除接口。

## 11. v1 不做

- 第三方登录（微信、Google）。
- 邮箱验证：邀请码已承担准入门槛。
- 设备指纹与生物识别。
- 会话并发数限制。

新增上述能力时，先评估是否引入新的信任链与合规义务，再决定实现方式。
