---
name: implement
description: "Implement one agreed slice under AGENTS.md with a minimal diff and real verification."
disable-model-invocation: true
argument-hint: "Spec, issue, or slice to implement"
---

# Implement

按已确认的 spec 或切片实施。一次只做一个切片；没有 spec 时，先要求用户给出验收条件。

## 1. 对齐输入

- 用户给了 issue/spec/切片：读取并复述标题与验收条件。
- 只给了模糊目标：先用 `/clarify` 或要求补充，不要猜。
- 读取 `AGENTS.md` 第 2、5 节，以及相关 `docs/`、ADR。
- 找到仓库的检查命令：优先 `bash scripts/check.sh all`；否则读 `Makefile`、`package.json`、CI 配置，不要假设技术栈。

开工前用 3–5 行说明：

```text
切片：...
验收：...
计划改动：...
验证命令：...
风险：...
```

## 2. 实施

- 最小改动；不顺手重构、不批量格式化、不引入无关依赖。
- 复用仓库既有结构、命名和工具。
- 需要新依赖时，先说明必要性、替代方案和成本。
- 不删除无关代码或测试；不通过跳过检查让结果变绿。
- 需要改文档、CHANGELOG 或 ADR 时，在同一个变更里完成。
- **不要** `git commit`、`git push` 或创建 PR，除非用户明确要求。

## 3. 验证

每个切片完成后：

1. 运行最相关的单项检查。
2. 运行仓库的全量检查。
3. 如果改动涉及界面、CLI 或脚本，实际执行一次并观察结果。
4. 记录命令与关键输出；失败时修根因，不隐藏。

## 4. 收尾

```markdown
## 改动
- 文件与行为

## 验证
- `命令` → 结果

## 未验证 / 风险
- ...

## 下一步
- 剩余切片或建议运行 `/review`
```

如果全部完成且存在 `review` 技能，建议调用它做最终审查。不要自行宣布“可以合并”。
