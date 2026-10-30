# ADR-0003：GitHub 为主、Gitee 对等的双平台 CI

- 状态：Accepted
- 日期：2026-10-30
- 决策者：模板维护者
- 相关：[CI/CD 说明](../ci-cd.md)

## 背景

仓库需要同时存在于 GitHub 与 Gitee。两个平台的 CI 能力不对等：

- GitHub Actions 有成熟的 Marketplace、hosted runner、Dependabot。
- Gitee 官方 CI 是 Gitee Go，YAML 存放在 `.workflow/`，执行环境与插件体系不同。
- 个人维护者不可能为每个检查写两套实现。

## 决策

采用“**共同核心 + 平台增强**”：

- 共同核心：`scripts/check.sh`，两端都执行。
- GitHub 增强：markdownlint、lychee、gitleaks、Dependabot。
- Gitee 增强：只跑共同核心，必要时后续加自建 Runner。
- GitHub 为主开发平台；Gitee 作为镜像与国内访问入口，不追求 100% 检查项对等。

## 后果

### 正面

- 规则实现只有一份，减少漂移。
- GitHub 端覆盖更完整；Gitee 端仍能阻断明显问题。
- 新增平台时只需接一个 `check.sh` 入口。

### 负面 / 代价

- 两端门禁强度不同，Gitee 可能放过 GitHub 能发现的问题。
- Gitee Go 首次配置需要手工在控制台操作。
- 平台语法变化需要分别维护两个 YAML。

### 风险与缓解

- 风险：开发者只走 Gitee，绕过 gitleaks/lychee。
- 缓解：文档明确差异；重要变更在 GitHub 合并后再同步。
- 风险：Gitee 镜像过旧导致脚本不可用。
- 缓解：核心脚本只依赖 POSIX 常用工具；在 Gitee 手动跑一次验证。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 只维护 GitHub | 简单 | Gitee 无门禁 | 不满足双平台要求 |
| 两端各写一套 | 平台原生 | 双倍维护，必然漂移 | 个人模板不可持续 |
| 自建 Runner 统一 | 环境完全一致 | 运维成本高 | 过度工程化 |
| 只维护 Gitee | 国内访问好 | 生态与工具少 | 开发体验差 |

## 验证方式

- GitHub：推送/PR 后检查三个 job 全绿。
- Gitee：手动执行流水线，确认日志出现 `All checks passed.`。
- 本地：`bash scripts/check.sh all`。
- 复核：平台文档或插件版本变化时更新 YAML。

## 参考

- GitHub Actions：<https://docs.github.com/en/actions>（访问日期：2026-10-30）
- Gitee Go 初识：<https://help.gitee.com/gitee-go/intro>（访问日期：2026-10-30）
- Gitee Go 流水线基本概念：<https://help.gitee.com/gitee-go/pipeline/basic-config>（访问日期：2026-10-30）
- Gitee Go 任务编排：<https://help.gitee.com/gitee-go/pipeline/scheduling>（访问日期：2026-10-30）
