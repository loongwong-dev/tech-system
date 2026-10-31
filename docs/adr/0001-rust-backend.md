# ADR-0001：后端采用 Rust + axum + sqlx

- 状态：Accepted
- 日期：2026-10-31
- 决策者：仓库维护者
- 相关：[AGENTS.md](../../AGENTS.md)、[平台需求](../product/platform/requirements.md)

## 背景

平台是面向教培教师的 AI 反馈工具，形态为多租户 SaaS。首年规模很小（租户 ≤ 300，峰值 < 50 RPS），但有三条硬约束：

- 单人开发，长期独立维护，可读性优先于初期速度。
- 多租户数据隔离是安全底线，不能依赖开发者自觉。
- 单机部署，不引入额外运行时或集群。

此前讨论中的默认方案是 TypeScript 全栈。仓库维护者决定后端改用 Rust，理由是类型系统与编译期校验能长期降低维护成本，且不需要为性能预留额外架构。

- 不做什么：不采用微服务、不引入第二套运行时、不使用需要额外基础设施的语言栈。

## 决策

后端固定为以下组合：

- 运行时：tokio
- Web：axum（依赖 tower / tower-http 中间件生态）
- 数据访问：sqlx，编译期校验 SQL
- 迁移：sqlx migrate，纯 SQL 文件
- 契约：utoipa 生成 OpenAPI
- 校验：serde + garde
- 日志：tracing
- 密码：argon2

必须满足的规则：

1. 所有查询通过 `sqlx::query!` / `query_as!` 或离线校验，禁止拼接 SQL 字符串。
2. 迁移只写 SQL 文件，不通过代码生成 DDL。
3. 领域错误使用 `thiserror`，`anyhow` 只出现在进程边界。
4. 不混用第二套数据访问方案。

## 后果

### 正面

- 租户隔离可以做成类型级约束，遗漏条件的查询编译不过。
- SQL 在编译期校验，字段改名会立刻暴露全部调用点。
- 单机性能余量极大，无 GC 停顿，内存占用可预测。
- 长期维护成本低，重构安全性高。

### 负面 / 代价

- 同等功能下开发速度比 TypeScript 慢 1.3–1.8 倍，里程碑按 Rust 重估。
- 生态缺口明显：没有 pg-boss、satori 这类开箱组件，队列与渲染需要自建（见 ADR-0002、ADR-0003）。
- 编译期 SQL 校验需要数据库连接或 `.sqlx` 离线缓存，CI 必须准备。
- AI 辅助对 Rust 的准确率低于 TypeScript，模型更容易编造 crate API。

### 风险与缓解

- 风险：AI 生成不存在的 API 导致反复试错。
  缓解：版本锁死在 `Cargo.lock`，不引入小众 crate，以 `cargo check` 作为高频回路。
- 风险：编译时间拖慢迭代。
  缓解：开发期只跑 `cargo check`；镜像构建使用 cargo-chef 分层缓存。
- 风险：单人维护 Rust 的认知负担过高，导致进度停滞。
  缓解：里程碑拆分到每周可交付的小块；若连续两个里程碑超期，重新评估语言选择并记录新的 ADR。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| TypeScript + Fastify + Zod + Drizzle | 开发最快、AI 辅助最准、生态最全 | 类型安全弱于 Rust，多租户约束依赖约定与测试 | 维护者明确选择 Rust |
| Go + chi + sqlc | 简单、编译快、部署方便 | 类型表达力弱，错误处理冗长 | 类型系统不足以表达租户约束 |
| NestJS | 约定强、模块化好 | 装饰器与 DI 增加间接层，运行时更重 | 单人项目收益为负 |

## 验证方式

- 命令或检查项：CI 执行 `cargo fmt --check`、`cargo clippy -- -D warnings`、`cargo test`、`cargo sqlx prepare --check`。
- 观察指标：单张反馈渲染与一次生成请求的端到端耗时；进程常驻内存。
- 复核时间：M5 上线后两周，以及每次新增 crate 依赖时。

## 参考

- axum 文档：<https://docs.rs/axum>（访问日期：2026-10-31）
- sqlx 文档：<https://docs.rs/sqlx>（访问日期：2026-10-31）
- utoipa 文档：<https://docs.rs/utoipa>（访问日期：2026-10-31）
- Rust ORM 对比（2026）：<https://www.rustfinity.com/blog/rust-orms>（访问日期：2026-10-31）
