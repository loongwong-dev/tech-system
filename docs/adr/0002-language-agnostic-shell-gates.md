# ADR-0002：语言无关的 Shell 门禁

- 状态：Accepted
- 日期：2026-10-30
- 决策者：模板维护者
- 相关：[CI/CD 说明](../ci-cd.md)、[配置说明](../configuration.md)

## 背景

模板不绑定语言/框架，但质量门禁必须能实际运行。常见方案各有依赖：

- markdownlint、commitlint、lychee 需要 Node/Rust 或额外二进制。
- Gitee Go 的 `build@gcc` 镜像较旧，不能假设最新 Node 可用。
- 个人模板不应为了“检查文档”引入完整的包管理和锁文件。

## 决策

核心门禁用 `scripts/check.sh` 实现，只依赖 bash、git 与 POSIX 常用文本工具（grep、awk、sed、find、sort 等）：

- 文档结构、H1、索引覆盖、ADR 命名、站内链接。
- 换行、Tab、行尾空白、末尾换行。
- Conventional Commits（支持 PR 范围与 Gitee 变量）。
- 轻量密钥特征扫描。

平台特定的重检查放在 GitHub Actions：

- markdownlint-cli2：完整 Markdown 规则。
- lychee：外链检查。
- gitleaks：完整历史密钥扫描。

## 后果

### 正面

- 任意 Linux/macOS 环境可跑，无安装步骤。
- Gitee 与本地使用同一套核心检查，结论一致。
- 规则可读、可改，不依赖黑盒 Action。

### 负面 / 代价

- Shell 脚本能力有限：不解析 Markdown AST，不检查锚点。
- 密钥扫描是基线，可能漏报或误报。
- 外链检查在 Gitee 端默认关闭，覆盖面小于 GitHub。

### 风险与缓解

- 风险：脚本随规则增多变得难维护。
- 缓解：按子命令拆分；新增复杂检查优先放平台 Action，不无限扩张 Shell。
- 风险：自研检查与标准工具行为不一致。
- 缓解：GitHub 端同时跑标准工具；以标准工具为准修复。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 全部用 GitHub Actions | 生态成熟 | Gitee 无法复用；本地无法离线 | 不满足双平台 |
| 全部用 pre-commit | 本地体验好 | 需 Python；CI 仍需另配 | 增加硬依赖 |
| Docker 统一镜像 | 环境一致 | 需要 Docker；Gitee 端配置更重 | 个人模板过重 |
| 只靠人工审查 | 零依赖 | 不可执行、不可审计 | 与目标冲突 |

## 验证方式

- 命令：`bash scripts/check.sh all`。
- 负向验证：制造坏链接、非规范提交、疑似密钥，确认退出码非零。
- 复核：每次新增规则时检查是否真的能失败。

## 参考

- Gitee Go 云端编译插件：<https://help.gitee.com/gitee-go/plugin/ci-build>（访问日期：2026-10-30）
- Gitee Go Shell 插件：<https://help.gitee.com/gitee-go/plugin/shell>（访问日期：2026-10-30）
- Conventional Commits：<https://www.conventionalcommits.org/zh-hans/v1.0.0/>（访问日期：2026-10-30）
- Write the Docs：Docs as Code：<https://www.writethedocs.org/guide/docs-as-code/>（访问日期：2026-10-30）
