# Goal: 实现 progress 工具，强制 commit agent 遵循步骤

## 状态：待实施

## 问题

LLM 在长 agent loop 中的典型失败模式：
1. **跳过验证步骤** — Step 6（Verify）最容易被跳过，LLM 厌恶自检
2. **合并步骤** — 把 Step 2-4 压缩成一个回合直接出摘要
3. **遗忘后段步骤** — tool call 积累后，prompt 开头的步骤指引权重衰减

仅靠 prompt 文字约束不够——模型会在生成过程中"忘记"中间步骤的要求。

## 方案：progress 工具 + 三重强制机制

### 工具 API

```json
{
  "step": "Step 1 — Load context",
  "status": "completed",
  "note": "3 cache files loaded — Rust CLI project using rig-core"
}
```

### 输出

格式化进度表，让模型一眼看到已完成/待做步骤：

```
✓ Step 1 — Load context           completed   (3 cache files loaded)
✓ Step 2 — Survey the diff        completed   (12 files, 340 lines, medium)
→ Step 3 — Classify and group     in_progress
· Step 4 — Contextual deep-dive   pending
· Step 5 — Subagent exploration   pending
· Step 6 — Verify the summary     pending
· Step 7 — Write the summary      pending
```

### 三重强制机制

| 层 | 机制 | 防御的失败模式 |
|----|------|---------------|
| 动作强制 | 必须主动调用工具才能标记进度 | 被动跳过步骤 |
| 反思强制 | `note` 必填，迫使完成前总结发现 | 标记 completed 但没认真完成 |
| 顺序强制 | 必须先 `in_progress` 再 `completed`，不可批量 | 一次性标记全部 completed |

### 内部校验规则（代码层最小强制）

| 规则 | 实现 | 防御的失败模式 |
|------|------|---------------|
| `status=completed` 时 `note` 必填 | 无 note 返回 ToolError，模型被迫重试 | 标记了但没做 |
| 同一步骤必须先 `in_progress` 再 `completed` | 状态机校验，违规返回错误 | 跳过实际工作直接标记 |
| 一次调用只能更新一个步骤 | 单步更新 | 批量标记跳步 |

### 状态管理

```rust
use std::sync::{Mutex, OnceLock};

enum StepState { Pending, InProgress, Completed }
struct StepStatus { state: StepState, note: Option<String> }

static PROGRESS: OnceLock<Mutex<HashMap<String, StepStatus>>> = OnceLock::new();

pub fn progress_state() -> &'static Mutex<HashMap<String, StepStatus>> {
    PROGRESS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn reset_progress() {
    progress_state().lock().unwrap().clear();
}
```

- 每次 `llm::commit()` 调用前 `reset_progress()`
- 单次 CLI 进程，`OnceLock` + `Mutex<HashMap>` 足够
- 不持久化到 `.git/vibe/`

### ToolLog 集成

`src/tool/mod.rs` 的 `ToolLog` match 添加：
```rust
"progress" => progress::show(args),
```

show 函数输出：`progress  → Step 3 — Classify and group` 或 `progress  ✓ Step 2 (12 files, medium)`

### 决策记录

| 决策 | 选项 | 选择 | 理由 |
|------|------|------|------|
| 作用域 | commit only / commit + summarize | commit only | 先验证效果再扩展 |
| 强制层级 | 仅 prompt / prompt + 代码校验 / 多轮编排 | prompt + 代码校验 | 工具内状态机是最小侵入的代码层强制 |
| 状态生命周期 | 每次重置 / 持久化 | 每次重置 | 单次分析无需持久化 |
| 多轮编排 | 暂不引入 | 分步走，先验证当前方案 | prepare-commit-msg hook 对延迟敏感 |

### 涉及文件

| 操作 | 文件 |
|------|------|
| 新建 | `docs/tool/progress.txt` |
| 新建 | `src/tool/progress.rs` |
| 修改 | `src/tool/mod.rs` |
| 修改 | `src/llm.rs` |
| 修改 | `docs/prompt/commit.txt` |

### 后续可选迭代

如果 prompt + 工具校验仍不够：
- **代码层结果校验**：agent 结束后检查 progress 状态，Step 6 未完成则追加 prompt 重试
- **多轮编排**：将 7 步拆成 2-3 个独立 LLM call，代码层面保证步骤执行
