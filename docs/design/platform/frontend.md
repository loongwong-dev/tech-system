# 前端架构

两个前端应用的边界、状态管理、离线草稿、微信内兼容与性能预算。

- **类型**：项目设计
- **覆盖范围**：共享边界、技术栈、路由、状态、草稿同步、流式渲染、微信兼容、性能预算。
- **不覆盖范围**：接口契约见 [API 契约](api-contract.md)；C 端形态决策见 [ADR-0007](../../adr/0007-responsive-web-client.md)。
- **状态**：已定，实现偏离时同步更新。

## 1. 两个应用与共享边界

| 项 | C 端 | 管理端 |
| --- | --- | --- |
| 使用者 | 教师，手机为主 | 运营，桌面 |
| 布局 | 响应式，`< 768px` 单列 | 侧边栏加表格 |
| 会话 | `sid` | `asid` |
| 首屏 JS 预算 | < 150KB gzip | < 500KB gzip |

共享范围只到基元：

| 包 | 内容 |
| --- | --- |
| `packages/ui` | 约 15 个基元组件与设计 token |
| `packages/api-types` | OpenAPI 生成的类型与请求函数，禁止手改 |

不共享表格、筛选栏、页面外壳等页面级组件。管理端的表格逻辑一旦进入共享包，C 端会被迫跟着升级依赖、扩大包体。

当前使用 shadcn/ui `radix-nova` + neutral 主题，品牌色待产品确定后再改 token。

`apps/web/components.json` 与 `packages/ui/components.json` 让 CLI 把组件写入共享 UI 包；CLI 生成后按仓库别名微调相对导入，不手写替代组件。CLI 当前 registry 直接依赖 `cn` 包；`packages/ui` 使用该包，`lib/utils` 仅保留给 CLI alias 兼容。

当前共享基元：`Button`、`Input`、`Textarea`、`Select`、`Label`、`Dialog`、`Separator`、`Field`/`FieldGroup`、`Toaster`。Tailwind 主题映射在 `packages/ui/src/styles.css`，C 端 `global.css` 只负责导入与页面级基础样式；组件源码位于 `packages/ui`，通过根 catalog 统一依赖版本。

## 2. 技术栈

| 项 | 选择 | 理由 |
| --- | --- | --- |
| 构建 | Vite | 两个独立应用，无 SSR 需求 |
| 路由 | TanStack Router | 路由参数类型安全 |
| 服务端状态 | TanStack Query | 缓存、重试、失效统一处理 |
| UI 基元 | shadcn/ui + Tailwind | 组件源码进仓库，样式可控 |
| 表单 | react-hook-form + Zod | 仅登录、注册、学员编辑使用 |
| 本地 UI 状态 | `useState` + URL 参数 | 不引入全局 store |
| 字体 | 系统字体栈 | 不加载中文 Web 字体 |

不引入全局状态库。筛选条件放 URL，可刷新可分享；服务端数据交给 Query 缓存；其余是局部状态。

## 3. 路由

C 端：

| 路径 | 页面 |
| --- | --- |
| `/login` | 登录 |
| `/register` | 邀请码注册 |
| `/select-tenant` | 租户选择 |
| `/` | 今日课次 |
| `/lessons/:id` | 课次录入与批量生成 |
| `/lessons/:id/review` | 审核与分享 |
| `/students` | 学员与班级 |
| `/settings` | 账号与配额 |

管理端：

| 路径 | 页面 |
| --- | --- |
| `/` | 健康页 |
| `/tenants` | 租户列表 |
| `/tenants/:id` | 租户详情 |
| `/invite-codes` | 邀请码 |
| `/audit` | 审计日志 |

除首页外全部路由级懒加载。

## 4. 离线草稿

教师常在弱网环境录入，丢一次数据就会流失。

```text
Key:   draft:{tenantId}:{lessonId}
Value: {
  savedAt: number,
  students: {
    [studentId]: { tags: string[], notes: string, updatedAt: number }
  }
}
```

| 环节 | 行为 |
| --- | --- |
| 编辑 | 先写内存，立即持久化到 IndexedDB |
| 同步 | 防抖 1 秒推送服务端；失败标记 dirty |
| 进入页面 | 本地与服务端合并；本地更新且未同步时提示 |
| 身份 | 本地草稿按 `studentId` 归并，不按反馈 id 建索引；服务端返回的反馈 `id` 为准，写入语义见 [API 契约](api-contract.md) 第 7.2 节 |
| 生成前 | 强制 flush，失败则阻止并提示 |
| 清理 | 该课次全部反馈确认后删除本地草稿 |

冲突处理采用按学生的最后写入优先。单用户场景不需要 CRDT。

浏览器不支持 IndexedDB 时降级到 localStorage；仍然失败则退化为纯内存，并在界面提示「本次修改不会保留」。

## 4.1 运行时边界

- 请求统一走 `apps/web/src/runtime/api.ts`：`credentials: include`，Problem Details 转 `ApiClientError`。
- Query key 集中在 `runtime/queryKeys.ts`；会话查询使用 `runtime/session.ts` 的 `meQuery`。
- 受保护路由通过 `beforeLoad: requireSession` 引导 `/me`；未登录与未选租户分别跳转登录页和租户选择页。
- 单份 SSE 解析在 `runtime/sse.ts`，按 `delta` / `done` 事件消费，支持超时与外部中断。
- 路由错误与加载态使用统一 `RouteError` / `RouteLoading`，不在页面内重复实现。

## 5. 流式生成

```text
fetch POST → response.body → TextDecoderStream → 逐段追加
```

| 情况 | 处理 |
| --- | --- |
| 用户离开 | 中断请求，保留已收内容 |
| 网络中断 | 标记未完成，可重新生成 |
| 服务端校验失败 | 显示原因，保留教师原始要点 |
| 重新生成 | 受 `regen_count` 限制，超过 3 次禁用 |

原则：已经展示给用户的内容不允许凭空消失。

## 6. 微信内置浏览器

| 现象 | 原因 | 对策 |
| --- | --- | --- |
| 返回上一页数据不刷新 | bfcache | 监听 `visibilitychange` 重新拉取 |
| 键盘弹起遮挡输入 | 视口高度变化 | 使用 `dvh`，输入框滚动进视口 |
| 长页面回弹 | iOS 滚动特性 | 避免嵌套滚动容器 |
| 下载链接无效 | 不支持 `download` | 图片走长按保存 |
| 样式不一致 | 内核版本较旧 | 不使用新特性做关键布局 |

## 7. 图片保存

微信内置浏览器无法可靠触发下载，唯一稳定路径是长按图片。

- 图片渲染为 `<img>`，不画在 canvas 中。
- 下方固定文案：「长按图片 → 保存到相册」。
- 批量导出逐张展示，不做压缩包。
- 始终保留「复制文字」作为零成本兜底。

## 8. 性能预算

| 项 | 目标 |
| --- | --- |
| C 端首屏 JS | < 150KB gzip |
| 首屏可交互 | 4G 下 < 2 秒 |
| 路由分割 | 必须 |
| 图片处理 | 全部在服务端完成 |

达到预算只有三条路：路由分割、按需引入、不引重型 UI 库。中文 Web 字体一律不用，用系统字体栈。

## 9. 请求层

统一封装三件事，业务代码里不应出现状态码判断：

| 场景 | 处理 |
| --- | --- |
| `application/problem+json` | 解析为统一的错误对象 |
| 401 | 清除本地会话状态并跳转登录 |
| 409 / 429 | 转为 Toast，展示 `detail` |

## 10. 目录

```text
apps/web/src/
  routes/     页面
  features/   按业务切分
  lib/        请求、IndexedDB、错误解析
  styles/     token 与全局样式
apps/admin/src/
  routes/
  features/
packages/ui/src/
packages/api-types/
```

按业务切分而不是按技术分层，避免出现一个巨大的 `components/` 目录。

## 11. 不做

- 不做 SSR 与静态生成。
- 不做全局状态库。
- 不做前端图像处理。
- 不做国际化。
- 不做微前端。
