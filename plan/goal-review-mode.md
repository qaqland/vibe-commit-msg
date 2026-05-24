# Goal: 新增 Review 模式 (`-r <hash>`)

## 状态：设计中

## 需求

新增 `-r <hash>` 选项，对指定 commit 引入的变更进行审阅。审阅范围是该 commit 与其第一个父 commit 之间的差异（`git diff <hash>^..<hash>`）。审阅流程与 commit agent 的前 6 步相似，但不生成最终的 commit message，而是在结构化摘要基础上增加审阅意见（潜在问题、改进建议）。

## 用户接口

```
vibe-commit-msg -r <hash>   Review a specific commit
```

- `<hash>` 可以是完整 SHA 或短 hash
- 输出到 stdout，格式为结构化摘要 + 审阅意见
- 不修改工作区、不修改 index、不写缓存

## 与 Commit 模式的对比

| 维度 | Commit 模式 | Review 模式 |
|------|------------|-------------|
| 数据源 | `git diff --cached`（staged changes） | `git diff <hash>^..<hash>`（已提交的单个 commit） |
| Tree 读取 | `git write-tree` → staged tree | `<hash>^{tree}` → 该 commit 的 tree |
| 流程 | Steps 1-7 → 生成 commit message | Steps 1-6 → 结构化摘要 + 审阅意见 |
| 输出 | commit message（写入 COMMIT_EDITMSG 或 stdout） | 结构化摘要 + 审阅意见（仅 stdout） |
| 缓存 | 可能触发缓存刷新 | 只读，不刷新缓存 |
| 副作用 | 无 | 无 |

## 架构设计

### 1. CLI 层 (`src/main.rs`)

新增 `Mode` 变体：

```rust
pub enum Mode {
    Commit(Option<PathBuf>),
    English(String),
    Help,
    Review(String),   // 新增
    Skip,
    Summary,
    Tool(String, String),
}
```

`-r` 解析逻辑：接收一个值参数作为 commit hash。

### 2. 工具层 (`src/tool/`)

核心问题：当前 diff/stat 硬编码 `git diff --cached`，read/list/grep 依赖 `STAGED_HASH`。review 模式需要不同的 git 数据源。

**设计思路**：启动时根据模式设置全局状态，工具内部透明切换，不改工具 API 参数。

- **Tree 来源**：`STAGED_HASH` 在 review 模式下被设置为 `<hash>^{tree}`，read/list/grep 自然工作
- **Diff 来源**：新增全局状态标记当前是否处于 review 模式及其 commit 引用，diff/stat 内部根据此标记选择 `git diff --cached` 或 `git diff <hash>^..<hash>`

具体实现方式待定（OnceLock / 环境变量 / 其他），但原则是：
- 工具 API 无参数变化
- 关心"当前是什么模式"的工具（diff/stat）能获取到该信息
- 不关心模式的工具（read/list/grep）零改动

### 3. 提示词层 (`docs/prompt/`)

新建 `review.txt`，独立于 `commit.txt`。

**与 commit.txt 的关系**：
- 共享步骤 1-6 的结构（Load context → Survey diff → Classify → Deep-dive → Subagent → Verify）
- Step 7 从"Write the summary"变为"Write the summary + review findings"
- 审阅特有的指令段追加在步骤之后

**审阅指令要点**（参考 OpenCode review.txt、sashiko、nitpicker、inspect）：

- 审阅焦点：bugs > structure > performance，与 commit agent 的"仅摘要"定位不同
- "Be certain" 原则：标记问题需要代码证据，不确定的标记 `[uncertain]`
- 不审阅未被修改的既有代码
- 不做风格洁癖——仅当违反项目既有范式时标注
- 行为变更必须标注（尤其可能是无意的）
- 不过度夸大严重性

**审阅输出格式**：

```
## Summary
[与 commit agent Step 7 相同的结构化摘要]

## Review
[审阅发现，按严重性排序]

### [Severity: error/warning/info]
- **Title**: 一行描述
  - Location: `/path/to/file:line` 或 `/path/to/file`
  - Scenario: 该问题如何在真实中被触发
  - Evidence: 具体代码引用
  - [uncertain] 如果推断不确定
```

### 4. LLM 层 (`src/llm.rs`)

新增 `review()` 函数：

```rust
pub async fn review(hash: &str) -> Result<String> {
    let preamble = prompt_description!("review");
    let agent = tool::agent()
        .preamble(preamble)
        .default_max_turns(DEFAULT_MAX_TURNS)
        .tool(Cache)
        .tool(Diff)
        .tool(Stat)
        .tool(Read)
        .tool(Grep)
        .tool(Glob)
        .tool(Subagent)
        .build();
    let prompt = format!("Review commit {}. Load the project memory cache first, then examine the diff and stat for this commit.", hash);
    let response = agent.prompt(prompt).await?;
    Ok(response.trim().to_string())
}
```

工具集与 commit agent 相同（Cache, Diff, Stat, Read, Grep, Glob, Subagent）。不包含 Log——审阅焦点是单 commit 引入的变更，不需要提交历史上下文。

### 5. 主流程 (`run()`)

```rust
Mode::Review(hash) => {
    // 1. 解析 hash → 完整 SHA
    // 2. 验证 commit 存在
    // 3. 设置 STAGED_HASH = <hash>^{tree}
    // 4. 设置 diff 模式标记（让 diff/stat 知道用 review 路径）
    // 5. 确保缓存目录存在（只读访问）
    // 6. 调用 llm::review(&hash)
    // 7. 输出到 stdout
}
```

### 6. 测试策略

- **工具层**：在 `tests/diff.bats` / `tests/stat.bats` 中新增测试——先创建多个 commit，然后用 `-t review-diff` / `-t review-stat` 工具传入目标 commit hash，验证输出正确
- **集成层**：新增 `tests/review.bats`，验证 `-r <hash>` 的端到端行为
  - 正常 commit 的审阅输出
  - 不存在的 hash 报错
  - merge commit（多父）的处理

### 7. 待决事项

| 事项 | 选项 | 备注 |
|------|------|------|
| diff/stat 模式切换的具体实现 | OnceLock / env var / 其他 | 原则已定（不改工具 API），实现细节留给实施阶段 |
| merge commit 的处理 | 只看第一个父 / 报错提示 | 需确认用户期望 |
| Review 模式是否需要 progress 工具 | 与 commit agent 同步 / 暂不需要 | 依赖 goal-progress-tool 的实施进度 |
| review.txt 是否引用 commit.txt 以减少重复 | 各自独立 / 共享步骤模板 | 独立更灵活，但步骤描述需手动同步 |
