# ADR-0016：主干开发启用自动合并，保留最新基线要求

- 状态：Accepted
- 日期：2026-11-02
- 决策者：仓库维护者
- 相关：[CI/CD 与质量门禁](../ci-cd.md)、[代码评审](../process/code-review.md)、[ADR-0010](0010-mandatory-review.md)、[CONTRIBUTING.md](../../CONTRIBUTING.md)

## 背景

`main` 是唯一长期分支，所有改动经 PR 合入。分支保护要求分支最新（`strict`）并通过 5 个必选检查，但决策前仓库未开启自动合并（`allow_auto_merge=false`）：CI 通过后仍需人工回到 PR 执行合并，等待集中在检查结束到手动合并之间。

2026-11-01 的测量结果：

- 近 30 次 PR 运行的 workflow 时长中位 95 秒，p90 172 秒，最长 298 秒；一次缓存命中的 `Rust` job 约 89 秒，是关键路径。
- 2026-10-31 合并 12 个 PR，合并频率约 15–60 分钟一次。

评估过两种「减少等待」的方案，均不成立：

- `develop` 分支：workflow 的 `pull_request` 触发器没有分支过滤，发往 `develop` 的 PR 仍跑同一组检查；总合并次数变成「feature→develop」加「develop→main」，反而增加。
- 合并队列：`loongwong-dev` 是 User 账号；GitHub 只支持组织拥有的公开仓库（或组织私有 + Enterprise Cloud）使用合并队列，当前仓库无法启用。

- 约束 1：单人维护，无法自我批准，合并只能由维护者执行（[ADR-0010](0010-mandatory-review.md)）。
- 约束 2：`main` 保持线性、逐 PR 绿色，变更可回溯到 Issue。
- 约束 3：不引入第二条长期分支，不为「消除等待」新增写权限机器人或新依赖。
- 不做什么：不引入 `develop`，不关闭 `strict`，不新增自动更新分支的 workflow。

## 决策

1. `main` 保持唯一长期分支，不引入 `develop`。
2. 开启仓库的 `Allow auto-merge`。维护者在 PR 通过评审后执行 `gh pr merge <N> --auto --squash`（或界面上的 Merge when ready），必选检查通过即自动合并。
3. 保留 `Require branches to be up to date before merging`（`strict`）与现有 5 个必选检查。
4. 多个 PR 同时打开且分支落后时，由维护者执行 `gh pr update-branch <N>`（需要时加 `--rebase`），再等待检查与自动合并。WIP=1 时通常不出现这种情况。
5. 自动合并只是维护者授权的执行方式；AI 不得执行合并或启用自动合并。

## 后果

### 正面

- 不再在 CI 结束后回到 PR 手动合并，等待从「检查 + 人工动作」缩短为「检查」。
- 保留 `strict`，每个 PR 仍基于最新 `main` 验证，不引入组合冲突。
- 零新增依赖、零新增 workflow 权限，改动可回退为一个仓库设置。

### 负面 / 代价

- 落后分支仍需人工更新，频率取决于同时打开的 PR 数量。
- 开启自动合并后，检查通过即合并；评审结论必须在启用之前给出。
- 仓库设置不随代码版本化，需要文档记录开启与回退方式。

### 风险与缓解

- 风险：评审前误开自动合并，改动未经人工确认即合入。
  缓解：流程规定先给出 `Approve` 再启用；单人仓库以 PR 评论记录评审结论。
- 风险：并发 PR 增多后，手动 `update-branch` 重新变成高频动作。
  缓解：保持 WIP=1；并发确实增加时再单独评估自动更新分支的 workflow。
- 风险：自动合并让「合并」动作不可见。
  缓解：PR 时间线与 commit 保留合并记录；维护者仍需在 PR 中留下结论。

## 备选方案

| 方案 | 优点 | 缺点 | 未采用原因 |
| --- | --- | --- | --- |
| 增加 `develop` 分支 | `main` 只在发布时更新 | CI 次数不变，总合并次数增加；需第二条保护线 | 不解决等待，只挪动等待位置 |
| 合并队列 | 组合验证，作者无需更新分支 | 仅组织拥有的仓库可用；当前为 User 账号 | 平台不支持 |
| 关闭 `strict` | 消除更新分支动作 | 两个单独绿的 PR 组合后可能破坏 `main` | 牺牲基线保证换取速度 |
| 自动更新分支 workflow | 保留 `strict` 且免手动更新 | 新增写权限与额外 CI 分钟 | 当前并发 PR 少，出现再评估 |

## 验证方式

- 命令或检查项：`bash scripts/check.sh all`；`gh api repos/loongwong-dev/tech-system --jq .allow_auto_merge` 返回 `true`。
- 观察指标：PR 从检查通过到 merged 的等待时间；落后分支手动更新的次数；需要回滚的合并数。
- 复核时间：启用后两周，或当前里程碑结束时。

## 参考

- GitHub：Automatically merging a pull request：<https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/automatically-merging-a-pull-request>（访问日期：2026-11-02）
- GitHub：Managing a branch protection rule（合并队列可用范围）：<https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/managing-a-branch-protection-rule>（访问日期：2026-11-02）
- GitHub：About protected branches（`strict` 与 `loose` 的取舍）：<https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-protected-branches/about-protected-branches>（访问日期：2026-11-02）
