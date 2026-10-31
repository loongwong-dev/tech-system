# 测试策略

单人维护项目的测试范围、分层与运行方式。

- **覆盖范围**：必测清单、测试分层、集成测试写法、常见失效陷阱、时间预算、CI 分工。
- **不覆盖范围**：CI 工作流配置见 [CI/CD 与质量门禁](../ci-cd.md)；业务规则见 [需求说明](../product/requirements.md)。
- **状态**：已定，新增业务表或状态机时同步更新必测清单。

## 1. 原则

测试的价值不在覆盖率数字，而在于拦住**靠肉眼发现不了**的失败。因此：

1. 只测会真实造成损失的路径，不为了覆盖率补测试。
2. 断言行为与错误码，不断言文案与内部实现。
3. 造数据直接用 SQL，不用走完整业务流程。
4. 每个测试独立，可并行执行。

## 2. 必测清单

| 优先级 | 测什么 | 失败后果 |
| --- | --- | --- |
| P0 | 跨租户访问返回 404 | 数据泄露 |
| P0 | 每张业务表都启用 RLS 且至少一条策略 | 新增表时遗漏 |
| P0 | 配额并发扣减不超发 | 收入与成本失控 |
| P0 | 队列并发唯一、崩溃回收、毒任务隔离 | 任务丢失或重复执行 |
| P1 | 反馈状态机拒绝非法转换 | 未审核直接分享 |
| P1 | 分享 token 过期与吊销 | 越权访问 |
| P1 | 质量闸门五条规则 | 幻觉与语气失真 |
| P1 | 标签到模板文本的映射 | 免费层兜底能力缺失 |
| P2 | 登录限流与账号锁定 | 暴力破解 |
| P2 | 邀请码原子消耗 | 超额注册 |

P0 的四项必须在 M0 完成时就有测试，不能等到上线前补。

## 3. 分层

| 层 | 工具 | 数量级 | 运行时机 |
| --- | --- | --- | --- |
| 单元 | `cargo test` | 大量 | PR |
| 集成 | `sqlx::test` | 30–50 | PR |
| 前端单元 | Vitest | 少量 | PR |
| 端到端 | Playwright | 3 条 | 合入 main 后 |

端到端只保留三条：注册到首次分享、批量生成、管理端开通权益。再多会成为维护负担，细节由集成测试覆盖。

测试文件放在哪里、何时选择同文件单元测试而非 `tests/` 集成测试，见 [代码规范](code-standards.md) 第 10 节。

## 4. 集成测试写法

`sqlx::test` 为每个测试创建独立数据库并执行全部迁移：

```rust
#[sqlx::test]
async fn tenant_cannot_read_other_tenant_feedback(pool: PgPool) {
    let (tenant_a, tenant_b) = seed_two_tenants(&pool).await;
    let feedback_b = seed_feedback(&pool, tenant_b).await;

    let result = fetch_feedback_as(&pool, tenant_a, feedback_b.id).await;

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().status(), 404);
}
```

两条纪律：

1. **造数据直接用 SQL**。走 API 会让测试慢一个数量级，并且把测试耦合到创建流程。
2. **不共享数据库**。独立建库的代价是慢，收益是并行安全。

RLS 策略覆盖测试用元数据查询完成，不需要逐表手写：

```sql
SELECT c.relname, c.relrowsecurity,
       (SELECT count(*) FROM pg_policy p WHERE p.polrelid = c.oid) AS policies
FROM pg_class c
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname = 'public' AND c.relkind = 'r';
```

断言业务表 `relrowsecurity` 为真且 `policies > 0`，例外清单中的表单独列出。

## 5. 常见失效陷阱

| 陷阱 | 后果 | 做法 |
| --- | --- | --- |
| E2E 依赖真实 AI 供应商 | 网络抖动导致误报 | 用固定响应替换模型调用 |
| 渲染测试逐像素比对 | typst 升级后全部变红 | 只断言尺寸与非空白 |
| 测试共用数据库 | 并行时随机失败 | 每个测试独立建库 |
| 断言错误文案 | 改一句提示就挂 | 断言错误码 |
| 时间相关断言用真实时钟 | 跨日运行失败 | 注入可控时钟 |

## 6. 不测什么

- 框架与库自身的行为（axum 路由、sqlx 类型转换）。
- 纯数据结构的 getter 与派生实现。
- 前端组件的每个渲染分支；只测有业务含义的交互。
- 视觉回归；当前阶段没有稳定的设计稿可对照。

## 7. 时间预算

测试代码大概是生产代码的 40–60%。8 周排期里应预留 1–1.5 周，而不是上线前集中补。

最值得投入的是 P0 四项：它们对应的缺陷一旦上线，修复代价远超写测试的时间。

## 8. CI 分工

| 触发 | 运行 |
| --- | --- |
| PR | `cargo fmt --check`、`cargo clippy -D warnings`、`cargo test`、前端单元 |
| PR | `cargo sqlx prepare --check`，确认查询缓存与代码一致 |
| 合入 main | 端到端三条 |
| 每日 | 全量测试 + 依赖安全检查 |

集成测试需要 PostgreSQL，CI 中启动服务容器，并设置 `DATABASE_URL`。本地使用同一份 Compose 定义，避免环境差异。
