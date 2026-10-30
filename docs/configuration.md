# 项目配置说明

本模板把配置分成三层：**必需基线**、**GitHub 平台配置**、**可选增强**。个人自用优先零依赖，不把可选工具写成硬门槛。

## 必需基线

| 文件 | 职责 | 修改注意 |
| --- | --- | --- |
| `AGENTS.md` | AI 行为契约 | 改规则需同步 CHANGELOG 与 ADR |
| `LICENSE` | Apache License 2.0 | 修改分发时保留许可证与版权声明 |
| `.editorconfig` | 跨编辑器文本格式 | 与 `check.sh format` 保持一致 |
| `.gitattributes` | 换行与二进制标记 | 新增二进制类型时补充 |
| `.gitignore` | 排除本地环境、产物、密钥 | 不要忽略应共享的规则文件 |
| `.markdownlint-cli2.jsonc` | Markdown 规则 | 放宽规则需说明理由 |
| `.lychee.toml` | 链接检查范围与超时 | 排除项越少越好 |
| `scripts/check.sh` | 语言无关门禁 | 新增子命令需更新文档 |
| `.github/workflows/quality.yml` | GitHub 门禁 | Action 版本由 Dependabot 更新 |
| `.github/ISSUE_TEMPLATE/*` | Issue 表单 | 表单字段变化时同步 |
| `.github/pull_request_template.md` | GitHub PR 模板 | 与 CONTRIBUTING 要求保持一致 |
| `CONTRIBUTING.md` | 人的协作流程 | 流程变化时更新 |
| `CHANGELOG.md` | 使用者可感知的变更 | 合并前写 `Unreleased` |
| `SECURITY.md` | 泄露与漏洞报告方式 | 联系方式可替换为仓库 Issue |

## GitHub

- `.github/workflows/quality.yml`：三 job（docs、links、secrets）。
- `.github/dependabot.yml`：每月检查 GitHub Actions 版本。
- Issue 表单：Bug、改进建议、文档问题、使用问题；`config.yml` 关闭空白 Issue。
- Issue 表单使用 YAML form，包含影响范围、环境、复现/验收等必填项。
- PR 模板路径：`.github/pull_request_template.md`。

## 可选增强

以下**不默认启用**，按需添加。它们都会增加运行时或网络依赖。

### pre-commit

适合本地提交前快速拦截，但需要 Python 环境和首次安装。个人模板不必默认引入。

```yaml
# .pre-commit-config.yaml（可选）
repos:
  - repo: https://github.com/pre-commit/pre-commit-hooks
    rev: v6.0.0
    hooks:
      - id: end-of-file-fixer
      - id: trailing-whitespace
        exclude: '\.md$'
      - id: check-yaml
  - repo: https://github.com/gitleaks/gitleaks
    rev: v8.30.1
    hooks:
      - id: gitleaks
```

启用后运行：

```bash
pre-commit install
pre-commit run --all-files
```

### commitlint

如果提交量变大，可把 `check.sh commits` 换成 commitlint。当前不引入 Node 依赖，因为 shell 正则已覆盖本模板需要的规则。

### CODEOWNERS

个人仓库没有强制评审需求，默认不添加。多人协作时再加 `.github/CODEOWNERS`：

```text
* @your-handle
```

### 文档站点

当前是纯 Markdown，不绑定静态站点生成器。需要站点时再选 MkDocs、VitePress 或 Docusaurus；不要在模板阶段预置。

## 版本与发布

模板本身使用语义化版本，但**不自动发版**：

1. 在 `CHANGELOG.md` 的 `Unreleased` 下记录变更。
2. 需要发布时，把 `Unreleased` 改为 `## [x.y.z] - YYYY-MM-DD`。
3. 打 tag：`vX.Y.Z`。
4. 不强制 GitHub Release；需要时再创建。

## 维护节奏

| 频率 | 动作 |
| --- | --- |
| 每次变更 | `bash scripts/check.sh all` |
| 每月 | 合并 Dependabot 的 Actions 更新；抽查外链 |
| 每季度 | 审计 `AGENTS.md` 规则是否仍可执行；清理过期排除项；按 [文档治理](governance.md) 审计文档 |
| 平台变更时 | 复核 GitHub Actions 语法与权限 |

文档维护与审计细节见 [文档治理与组织](governance.md)。

## 参考

- EditorConfig：<https://editorconfig.org/>
- Git attributes：<https://git-scm.com/docs/gitattributes>
- Conventional Commits：<https://www.conventionalcommits.org/zh-hans/v1.0.0/>
- Keep a Changelog：<https://keepachangelog.com/zh-CN/1.1.0/>
- ADR：<https://adr.github.io/>

访问日期：2026-10-30。
