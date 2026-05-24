# Goal: 优化 commit agent 提示词

## 状态：已完成（已写入 docs/prompt/commit.txt）

## 问题

原 `commit.txt` 5 步流程存在以下缺陷：
- 噪声过滤仅覆盖 lock file，遗漏 generated/vendored/auto-managed
- 无规模感知，小 diff 走完整流程浪费 token，大 diff 缺乏深层探索
- "related changes" 分组定义模糊，模型行为不一致
- 无统一主题时无处理指引
- diff 歧义时缺少"必须读上下文"的强制
- 描述只说 what 不说 why
- 无推测性标注机制，不确定的信息直接输出
- subagent 已配置但提示词未提及使用场景
- 无验证步骤，LLM 倾向跳过自检

## 方案：5 步 → 7 步重写

| Step | 原 | 新 | 关键变更 |
|------|----|----|---------|
| 1 | Load context | Load context | 不变 |
| 2 | Examine the diff | Survey the diff | 噪声过滤扩展 + 规模评估（小/中/大） |
| 3 | Classify and filter | Classify and group | (a)(b)(c) 三条件精确定义 related + 多主题处理 |
| 4 | Deep-dive as needed | Contextual deep-dive | "Diffs alone are not enough" + "Why > What" + `[uncertain]` 标注 |
| 5 | Write the summary | Subagent exploration | 新增：使用时机 + quick/medium/thorough 深度级别 |
| — | — | Verify the summary | 新增：自检验证（证据、推测标记、分组、标识符精确性） |
| — | — | Write the summary | 支持 `[uncertain]` 标注 + 多主题 |

## commit.txt 新增 Step tracking 段落（待 progress 工具实施后启用）

```
Step tracking:
  You MUST use the `progress` tool to track your progress through all steps.
  - Before starting each step: call progress with status="in_progress"
  - After completing each step: call progress with status="completed" and a
    note summarizing what you found or decided (note is REQUIRED)
  - Do not batch-update multiple steps — call progress for each step as you
    work through them
  - Step 5 may be skipped for small diffs, but all other steps are mandatory
```

## 变更来源追踪

| 变更 | 来源参考 |
|------|---------|
| 噪声过滤扩展 | inspect diff 截断策略 |
| 规模评估 | repo-analyzer 深度模式 |
| 分组定义精确化 | repo-analyzer 业务分组法则 |
| 多主题处理 | 审查发现的 gap |
| "Diffs alone are not enough" | OpenCode review.txt |
| "Why > What" | repo-analyzer 核心原则 |
| `[uncertain]` 标注 | sashiko/nitpicker 验证机制 |
| Subagent 探索 | issue #4, Claude Code, nitpicker |
| Verify 验证步骤 | sashiko 交叉验证, inspect 代理挑战 |
