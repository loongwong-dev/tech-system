# 代码规范

目录组织、模块边界、文件与函数规模、可见性、错误处理与自动检查。

- **覆盖范围**：Rust 后端与 TypeScript 前端的组织方式与可检查规则。
- **不覆盖范围**：提交信息见 [CONTRIBUTING.md](../../CONTRIBUTING.md)；测试范围见 [测试策略](testing.md)。
- **状态**：已定，规则编号稳定，删除或改号需同步更新引用。

## 1. 原则

1. **一个文件只有一个变更理由。** 行数只是代理指标；真正的判据是「什么时候需要改它」。
2. **边界用编译器表达，不用纪律表达。** 能靠 crate 依赖或类型系统拦住的，不写进评审清单。
3. **规则必须可检查。** 每条规则要么有命令，要么有明确的评审动作。

## 2. 工作区与依赖方向

```text
Cargo.toml              # workspace：统一版本、lint、edition
crates/
  shared/               错误类型、配置、ID 与通用校验
  db/                   连接池、迁移、仓储基元
  domain/               学员、班级、课次、反馈、分享
  platform/             租户、账号、会话、权限、配额、审计
  ai/                   模型适配、prompt、质量闸门
  render/               typst 调用与缓存
  jobs/                 队列、worker、定时任务
  api/                  axum 装配、路由、中间件
apps/
  web/                  C 端
  admin/                管理端
packages/
  ui/                   设计系统
  api-types/            OpenAPI 生成物
```

依赖只能向下，不允许回环：

```text
api → jobs → domain → db → shared
api → platform → db → shared
api → ai, render → shared
```

| 规则 | 说明 |
| --- | --- |
| 禁止 `domain` 依赖 `api` 或 `platform` | 业务模型不认识 HTTP 与账号体系 |
| 禁止 `ai`、`render` 依赖 `db` | 它们是纯计算，输入输出都是结构体 |
| 禁止 `shared` 依赖任何其他 crate | 它是依赖树的叶子 |
| 前端只能消费 `packages/api-types` | 生成物是唯一契约 |

所有 crate 平铺在 `crates/` 下，不允许嵌在别的 crate 的 `src/` 里。

### 2.1 前后端共存

根目录同时是两个 workspace 的根，不再套一层 `frontend/`、`backend/`。两个生态各自读自己的清单，互不感知。

```text
Cargo.toml            Rust workspace 虚拟清单
Cargo.lock            提交
rust-toolchain.toml   工具链版本
rustfmt.toml
clippy.toml           可选
deny.toml             cargo-deny 配置

package.json          JS workspace 根，private，只放 workspaces 与脚本
bun.lock              提交，文本格式可 review
bunfig.toml           可选，安装策略与 registry
tsconfig.base.json

crates/               Rust 包
apps/                 前端应用
packages/             共享包
target/               忽略
node_modules/         忽略
```

| 文件 | 读取者 | 规则 |
| --- | --- | --- |
| 根 `Cargo.toml` | Cargo | 虚拟清单，只放成员、`[workspace.dependencies]`、`[workspace.lints]` |
| `crates/*/Cargo.toml` | Cargo | 依赖写 `.workspace = true`，不重复版本 |
| `Cargo.lock` | Cargo | 可执行程序必须提交，保证可复现 |
| 根 `package.json` | Bun | `private: true`，不写业务依赖 |
| `apps/*/package.json` | Bun | 只声明自身依赖，内部包用 `workspace:*` |
| `bun.lock` | Bun | 必须提交，禁止提交 `bun.lockb` |
| `bunfig.toml` | Bun | 安装策略、registry、脚本白名单 |

安装与构建约定：

1. 只在根目录执行 `bun install`，不在 `apps/` 下单独安装。
2. 新仓库的 Bun 默认使用 isolated 链接模式（每个包独立 `node_modules`，用符号链接关联），能拦住「用了却没声明」的幽灵依赖。团队内不要混用 `--linker hoisted`，否则两边的产物不一致。
3. 生产环境**不需要 Bun 运行时**：前端产物是静态文件，由 Caddy 直接提供。Bun 只用于开发、CI 与脚本。
4. 服务端是 Rust，Bun 不参与任何线上请求路径。
5. Vite 用 Bun 执行脚本即可；若插件出现运行时兼容问题，退回 Node 执行，不影响安装与依赖管理。

## 3. 目录与模块

顶层按业务切分，模块内部才按层次切分：

```text
crates/domain/src/feedback/
  feedback.rs            mod 声明与对外导出
  feedback/model.rs      数据结构与状态机
  feedback/repository.rs 仅 SQL
  feedback/service.rs    业务规则
  feedback/error.rs      本模块错误
```

| 文件 | 只允许放什么 | 禁止 |
| --- | --- | --- |
| `model.rs` | 结构体、枚举、状态转换 | 不碰数据库、不碰 HTTP |
| `repository.rs` | SQL、行映射 | 不写业务分支 |
| `service.rs` | 业务规则、事务边界 | 不直接构造 HTTP 响应 |
| `routes.rs` | 请求解析、响应映射 | 不写业务规则 |
| `error.rs` | 本模块错误枚举 | 不吞掉底层错误 |

用 `feedback.rs` 加 `feedback/` 目录，不用 `feedback/mod.rs`。前者让父文件承担「模块说明与导出」，后者容易变成堆积处。

模块目录文件数超过 8 个时再分组；在此之前不预建空目录。

## 4. 规模上限

| 对象 | 软上限 | 硬上限 | 超出后 |
| --- | --- | --- | --- |
| 文件 | 300 行 | 500 行 | 拆分，或首行写 `// code-size:allow` 并说明理由 |
| 函数 | 50 行 | 80 行 | 拆成命名清晰的子步骤 |
| 嵌套 | 3 层 | 4 层 | 提前返回、`?` 传播、提炼函数 |
| 函数参数 | 4 个 | 5 个 | 改为结构体参数 |
| 公开方法（单类型） | 10 个 | 15 个 | 说明是否承担了多个职责 |
| 模块公开项 | 15 个 | 20 个 | 拆模块 |

行数统计排除空行、导入与 `#[cfg(test)]` 之后的测试代码。检查命令见第 13 节。

数字触发的是**复核动作**，不是自动重构。超过软上限时应在评审中说明该文件为何仍只有一个变更理由。

## 5. 可见性与 API 表面

1. 默认私有，跨文件用 `pub(crate)`，跨 crate 才用 `pub`。
2. 每个 crate 只有一个出口文件（`lib.rs`），只导出确实需要对外的东西。
3. 禁止为了「以后可能用到」而 `pub`。
4. `pub use` 只用于重导出核心类型，不做通配导出。
5. 结构体字段默认私有，提供构造函数或构造器方法。

## 6. 命名

遵循 Rust 官方命名（RFC 430）与 API Guidelines：

| 对象 | 规则 | 示例 |
| --- | --- | --- |
| crate / 模块 | `snake_case` | `feedback` |
| 类型 / trait | `UpperCamelCase` | `FeedbackService` |
| 函数 / 变量 | `snake_case` | `generate_feedback` |
| 常量 | `SCREAMING_SNAKE_CASE` | `MAX_REGEN_COUNT` |
| 布尔 | `is_` / `has_` / `can_` 前缀 | `is_approved` |
| 错误枚举 | 动词或名词加 `Error` | `GenerateError` |

领域词汇全项目统一，不出现同义词：

| 概念 | 唯一写法 | 禁止 |
| --- | --- | --- |
| 课次 | `lesson` | `session`、`class_session` |
| 反馈 | `feedback` | `comment`、`report` |
| 学员 | `student` | `child`、`learner` |
| 租户 | `tenant` | `org`、`account` |

缩写只允许 `id`、`url`、`http`、`api`、`db`，其余写全。

## 7. 错误处理

| 规则 | 做法 |
| --- | --- |
| 每个模块一个错误枚举 | `thiserror` 派生，变体携带来源错误 |
| `anyhow` 只在进程边界 | `main`、worker 顶层，不进入业务函数 |
| 禁止 `unwrap` / `expect` | 请求路径、任务路径一律禁止；测试可用 |
| 错误必须带上下文 | 不允许 `map_err(\|_\| MyError::Unknown)` 丢弃原因 |
| 对外错误与内部错误分离 | HTTP 层映射为 `problem+json`，不暴露内部细节 |

## 8. 异步与并发

1. 禁止在 async 任务中执行阻塞操作；CPU 密集与子进程调用走 `spawn_blocking` 或独立 worker。
2. 锁不得跨 `await` 持有。
3. 所有外部调用必须有超时，不允许无限等待。
4. 后台任务必须幂等；至少一次投递是默认语义。

## 9. 数据访问

1. SQL 只出现在 `repository.rs`，其他层不出现 SQL 字符串。
2. 使用 `sqlx::query!` / `query_as!`，禁止字符串拼接构造 SQL。
3. 事务边界在 `service.rs`，仓储函数接受 `&mut Transaction`。
4. 仓储函数必须显式接收 `tenant_id`，不从上下文隐式获取。

## 10. 测试文件的位置

三种测试、三种位置，判据只有一个：**是否需要访问私有项**。

| 形态 | 位置 | 可访问私有项 | 进入发布构建 |
| --- | --- | --- | --- |
| 单元测试 | 与被测文件同一个 `.rs`，末尾 `#[cfg(test)] mod tests` | 是 | 否 |
| 集成测试 | crate 根目录的 `tests/` | 否 | 否 |
| 文档测试 | `///` 注释中的示例 | 否 | 否 |

```text
crates/domain/src/feedback/
  feedback.rs
  feedback/service.rs       ← 末尾写 #[cfg(test)] mod tests
  feedback/repository.rs
  feedback/model.rs
crates/domain/tests/
  feedback_generation.rs    ← 集成测试，需要真实数据库
  common/
    mod.rs                  ← 共享夹具，不能叫 common.rs
crates/api/tests/
  cross_tenant.rs
  auth_flow.rs
```

**单元测试写在同文件**，因为：能测 `pub(crate)` 与私有函数；读代码不用跳文件；`#[cfg(test)]` 不进入发布构建；这也是 Rust 官方惯例。

**集成测试单独放**，因为：`tests/` 下每个文件编译成独立 crate，强制只使用公开 API；与生产代码编译隔离，改测试不会触发全量重编译。

| 编号 | 规则 |
| --- | --- |
| CS-21 | 单元测试置于被测文件末尾的 `#[cfg(test)] mod tests` |
| CS-22 | 集成测试放 crate 根目录 `tests/`，一个文件一个业务场景 |
| CS-23 | 共享夹具放 `tests/common/mod.rs`，禁止 `tests/common.rs` |
| CS-24 | 测试文件不参与行数门禁，但仍遵守「一个文件一个场景」 |

常见错误：

| 错误 | 后果 |
| --- | --- |
| 把夹具写成 `tests/common.rs` | Cargo 视其为独立测试目标，运行时报告 0 个测试 |
| 在 `#[cfg(test)]` 中放生产辅助函数 | 该函数只在测试编译时存在，被别处引用会编译失败 |
| 所有集成测试塞进一个 `integration.rs` | 与「一个文件一个变更理由」直接冲突 |
| 用全局可变状态在测试间共享数据 | 并行执行时随机失败 |

文档测试适合纯函数：标签到话术的映射、质量闸门的规则判断。涉及数据库与 IO 的一律不用。

前端对应约定：单元测试与源文件同目录（`Button.test.tsx`），端到端测试放 `apps/web/e2e/`，快照由工具生成并与测试同目录存放。

## 11. 注释与文档

| 对象 | 要求 |
| --- | --- |
| 模块 | `//!` 说明职责与边界，一到三行 |
| 公开项 | `///` 必须，说明做什么与失败条件 |
| 私有函数 | 只在逻辑不直观时写 |
| TODO | 必须带跟踪编号：`// TODO(#12): …` |

注释解释「为什么」，不解释「做了什么」。

## 12. 依赖管理

1. 版本统一在 `[workspace.dependencies]`，crate 内只写 `.workspace = true`。
2. 新增依赖必须在 PR 中说明：解决什么问题、为何不用标准库、维护状态。
3. `cargo-machete` 检查未使用依赖。
4. `cargo-deny` 检查许可证与安全公告。
5. 依赖树里出现两个版本的同一 crate 时，必须说明原因。

## 13. 前端

| 规则 | 值 |
| --- | --- |
| 文件上限 | 300 行（软），500 行（硬） |
| 一个文件一个组件 | 组件与文件名一致 |
| 组件内禁止直接请求 | 一律走 `lib/api` |
| 导出方式 | 具名导出，不用 default export |
| 业务目录 | 按 feature 切分，不建全局 `components/` |
| 类型 | 只用 `packages/api-types`，不手写接口类型 |

## 14. 自动检查

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo machete
cargo deny check advisories licenses bans sources
bash scripts/check-size.sh
```

## 15. 规则总表

| 编号 | 规则 | 检查方式 |
| --- | --- | --- |
| CS-01 | crate 平铺于 `crates/`，不嵌套 | 评审 |
| CS-02 | 依赖方向符合第 2 节，禁止回环 | 编译期 |
| CS-03 | 一个文件只有一个变更理由 | 评审 |
| CS-04 | 文件 ≤ 500 行（测试与空行不计） | `check-size.sh` |
| CS-05 | 函数 ≤ 80 行 | clippy `too_many_lines` |
| CS-06 | 嵌套 ≤ 4 层 | 评审 |
| CS-07 | 参数 ≤ 5 个 | clippy `too_many_arguments` |
| CS-08 | 默认私有，跨 crate 才 `pub` | 评审 |
| CS-09 | 领域词汇唯一 | 评审 |
| CS-10 | 模块级 `//!` 说明 | clippy `missing_docs` |
| CS-11 | 公开项有 `///` | clippy `missing_docs` |
| CS-12 | 请求路径禁用 `unwrap` / `expect` | clippy `unwrap_used` |
| CS-13 | `anyhow` 不出现在业务函数 | clippy `disallowed_types` |
| CS-14 | SQL 只在 `repository.rs` | 评审 |
| CS-15 | 外部调用必须有超时 | 评审 |
| CS-16 | 测试命名描述行为 | 评审 |
| CS-17 | 依赖版本统一在 workspace | Cargo lint |
| CS-18 | 无未使用依赖 | `cargo machete` |
| CS-19 | 许可证与公告合规 | `cargo deny` |
| CS-20 | 前端文件 ≤ 500 行 | `check-size.sh` |
| CS-21 | 单元测试在被测文件内 | 评审 |
| CS-22 | 集成测试放 `tests/`，一文件一场景 | 评审 |
| CS-23 | 共享夹具用 `tests/common/mod.rs` | 评审 |
| CS-24 | 测试文件不计入行数门禁 | `check-size.sh` |
| CS-25 | 两个 lockfile 只在根目录，禁止子目录另生成 | 评审 |
| CS-26 | 依赖版本只在 workspace 根声明 | Cargo lint / 评审 |
| CS-27 | 只在根目录执行安装；子目录 `node_modules` 由工具生成，不提交不手工管理 | 评审 |

## 16. 评审清单

提交前逐条确认，只列机器查不出来的：

1. 新增文件是否只承担一个变更理由，文件名能否说明它。
2. 模块边界是否被绕过（例如 service 里直接写 SQL）。
3. 新增依赖是否说明了理由。
4. 错误是否丢失了原因。
5. 跨租户路径是否有测试覆盖。
6. 是否存在「暂时先用着」的临时代码。
