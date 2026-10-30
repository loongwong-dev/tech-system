---
name: handoff
description: "Compact the current session into a handoff document for a fresh agent."
disable-model-invocation: true
argument-hint: "What the next session will focus on"
---

# Handoff

把当前会话压缩成一份可交接文档，让新代理不依赖旧上下文也能继续。

## 存放位置

写入系统临时目录，不要写进工作区：

- macOS/Linux：`$TMPDIR`，没有则 `/tmp`
- Windows：`%TEMP%`

文件名：`handoff-YYYY-MM-DD-HHMM.md`。

## 内容

```markdown
# Handoff：<主题>

## 目标
这次会话要达成什么；当前完成到哪一步。

## 当前状态
- 已完成：
- 进行中：
- 未开始：

## 关键决策
- 决策 — 理由 — 影响

## 相关文件
- 路径 — 作用（只列真正需要的）

## 验证
- 已运行：命令 → 结果
- 未运行：原因

## 未决问题
- ...

## 下一步
1. 具体动作
2. 具体动作

## 建议技能
- `/implement` / `/review` / ...

## 警告
- 密钥、权限、破坏性操作、环境差异；没有则写“无”
```

## 规则

- 不复制 spec、ADR、diff 的全文；引用路径或 URL。
- 删除敏感信息：令牌、密码、个人信息、内部地址。
- 如果用户给了参数，把它当作下一次会话的重点，调整“下一步”和“建议技能”。
- 文档写完后，在回复中给出路径和 3 条以内摘要。
