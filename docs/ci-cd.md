# CI/CD 与质量门禁

目标不是“有一个绿勾”，而是让**可机检的规范真的被检查**，并且本地、GitHub、Gitee 三处的结论尽量一致。

## 门禁矩阵

| 检查 | 本地 | GitHub Actions | Gitee Go | 失败时先看 |
| --- | --- | --- | --- | --- |
| Markdown 规则 | `npx markdownlint-cli2`（可选） | `markdownlint-cli2-action` | 由 `scripts/check.sh docs` 覆盖结构，不跑完整规则 | `.markdownlint-cli2.jsonc` |
| 文档结构 / H1 / 索引 / ADR 命名 | `check.sh docs` | `check.sh docs` | `check.sh docs` | 缺失文件或未登记索引 |
| Skill frontmatter / 索引 / 调用策略 | `check.sh skills` | `check.sh skills` | `check.sh skills` | frontmatter、目录名或元数据不一致 |
| 站内链接 | `check.sh docs` | `check.sh docs` + lychee | `check.sh docs` | 相对路径或锚点 |
| 外链 | `CHECK_EXTERNAL_LINKS=1 check.sh links` | lychee | 默认不跑，避免网络波动 | 目标站点状态、排除项 |
| 密钥扫描 | `check.sh secrets`（基线） | gitleaks | `check.sh secrets`（基线） | 泄露位置、是否需轮换 |
| 提交信息 | `check.sh commits` | 按 PR/推送范围检查 | 按 PR 标题或最后一次提交检查 | Conventional Commits 格式 |
| 文本格式 | `check.sh format` | `check.sh format` | `check.sh format` | CRLF、Tab、末尾换行 |
| 依赖更新 | 手动 | Dependabot（每月） | 手动 | Actions 版本 |

> “基线”表示轻量正则扫描，不能替代 gitleaks。个人仓库的 GitHub 端已启用 gitleaks；Gitee 端受运行环境限制，只做基础特征匹配。

## 本地运行

```bash
bash scripts/check.sh all
```

分项运行：

```bash
bash scripts/check.sh docs
bash scripts/check.sh skills
bash scripts/check.sh format
bash scripts/check.sh commits
bash scripts/check.sh secrets
CHECK_EXTERNAL_LINKS=1 bash scripts/check.sh links
```

`check.sh` 只依赖 bash、git 与 POSIX 常用文本工具（grep、awk、sed、find、sort 等）。需要完整 Markdown 规则时：

```bash
npx markdownlint-cli2 "**/*.md"
```

## GitHub Actions

文件：`.github/workflows/quality.yml`。

- `docs`：checkout 全历史 → markdownlint → `check.sh docs/skills/format/commits`。
- `links`：lychee 按 `.lychee.toml` 检查外链与站内链接。
- `secrets`：gitleaks 扫描完整历史。
- 触发：推送到 `main`、所有 PR、手动触发。
- 权限：默认 `contents: read`；gitleaks job 需要 `pull-requests: read` 来读取 PR 提交列表，关闭 PR 评论以避免申请写权限。
- 组织账号使用 gitleaks 需要免费 license，并配置 `GITLEAKS_LICENSE` secret；个人账号不需要。
- 并发：同一分支的新运行会取消旧运行。

提交范围通过 `BASE_SHA` / `HEAD_SHA` 传入：

- PR：base SHA..head SHA。
- push：`github.event.before..github.sha`；首次推送的 before 全零时退化为只查 HEAD。

## Gitee Go

文件：`.workflow/quality.yml`。

Gitee 官方 CI 是 **Gitee Go**，流水线 YAML 存放在仓库 `.workflow/` 目录。配置包含：

- 触发：`main`/`master` 的 push 与 PR。
- 执行环境：`build@gcc`（Ubuntu 基础镜像，含 git、bash 等常用工具）。
- 任务：`bash scripts/check.sh all`。
- 阻塞构建：开启，避免同一流水线并发污染。

### 首次启用

1. 在 Gitee 仓库页面开通 Gitee Go（账号需绑定手机号）。
2. 新建流水线；若控制台支持选择仓库内 YAML，选择 `.workflow/quality.yml`。若当前 UI 只提供模板，选择任意模板创建后，把生成的 `.workflow/*.yml` 替换为本文件内容并保存。
3. 手动执行一次，确认日志中出现 `All checks passed.`。
4. 之后 push / PR 自动触发。

> 平台 UI 与“是否自动发现已有 YAML”可能随版本变化；以 Gitee Go 控制台当前行为为准。模板不假设存在 GitHub 式 hosted runner。

### Gitee 端限制

- 没有与 GitHub Actions 等价的 Marketplace；复杂检查需要自己装二进制或拆到自建 Runner。
- `build@gcc` 镜像版本较旧（Ubuntu 20.04 系），不要依赖最新 Node/Python 特性。
- 外链检查默认关闭，避免 CI 因目标站点反爬或网络波动失败。
- 提交信息在 PR 场景读取 `GITEE_COMMIT_MESSAGE`（PR 标题）；push 场景只查最后一次提交。

## 新增一个门禁

1. 先判断能否用现有 `check.sh` 子命令完成；能则不加新工具。
2. 需要新工具时，优先选单二进制、无运行时依赖、可在 Ubuntu 20.04 运行的工具。
3. 在 `scripts/check.sh` 增加子命令，并保证失败时退出码非零。
4. 同步更新：本文件矩阵、`docs/configuration.md`、`README.md` 的检查说明。
5. 若引入新依赖或平台能力，补 ADR。

## 排障顺序

1. 本地复现：`bash scripts/check.sh <子命令>`。
2. 看首个错误，不要先改配置放宽规则。
3. 链接失败：确认是否为 403/429、是否需要排除、是否应改为官方稳定链接。
4. 密钥失败：先确认是否真实泄露；真实泄露需轮换，而不是只删文件。
5. 平台差异：对比本地与 CI 的工具版本、工作目录、换行设置。

## 参考

- GitHub Actions 文档：<https://docs.github.com/en/actions>
- Gitee Go 初识：<https://help.gitee.com/gitee-go/intro>
- Gitee Go 流水线基本概念：<https://help.gitee.com/gitee-go/pipeline/basic-config>
- Gitee Go 触发事件：<https://help.gitee.com/gitee-go/pipeline/trigger>
- Gitee Go 任务编排：<https://help.gitee.com/gitee-go/pipeline/scheduling>
- Gitee Go 云端编译插件：<https://help.gitee.com/gitee-go/plugin/ci-build>
- markdownlint-cli2：<https://github.com/DavidAnson/markdownlint-cli2>
- lychee：<https://lychee.cli.rs/>
- gitleaks：<https://github.com/gitleaks/gitleaks>

访问日期：2026-10-30。
