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
| 数据源 | staged changes（index → tree） | 已提交的单个 commit |
| 流程 | Steps 1-7 → 生成 commit message | Steps 1-6 → 结构化摘要 + 审阅意见 |
| 输出 | commit message（写入 COMMIT_EDITMSG 或 stdout） | 结构化摘要 + 审阅意见（仅 stdout） |
| 缓存 | 可能触发缓存刷新 | 只读，不刷新缓存 |
| 副作用 | 无 | 无 |

## 架构设计

### 前置决策：引入 git2，用 Source enum 统一数据源

**核心洞察**：commit 模式和 review 模式的工具层需求完全相同——都需要一个 tree 和一个 diff。区别仅在数据来源。用 git2 的结构化 API 可以将两种模式收敛到一个统一的 `Source` 抽象，工具层零分支。

#### 为什么不用 shell-out + 虚拟 commit

shell-out 下统一两种模式的可行方案是构造一个虚拟 commit（`git commit-tree`），让 diff/stat 统一走 `git diff <hash>^..<hash>`。但这是 workaround——为统一命令行接口凭空造了一个 commit 对象（dangling object，需 gc 清理），且初始提交需要特殊 diff 语法。

#### git2 + Source enum 方案

```rust
pub struct Source {
    pub tree: Tree,               // read / list / grep / glob 用
    pub parent_tree: Option<Tree>, // diff / stat 用（None = 对空树比较）
}
```

两种模式的初始化：

```rust
// commit 模式：从 index 构建
fn from_staged(repo: &Repository) -> Source {
    let mut index = repo.index().unwrap();
    index.write_tree().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let parent_tree = repo.head().ok()
        .and_then(|h| h.peel_to_tree().ok());
    Source { tree, parent_tree }
}

// review 模式：从 commit 解析
fn from_commit(repo: &Repository, hash: &str) -> Source {
    let commit = repo.revparse_single(hash).unwrap().peel_to_commit().unwrap();
    let tree = commit.tree().unwrap();
    let parent_tree = commit.parent(0).ok().map(|p| p.tree().unwrap());
    Source { tree, parent_tree }
}
```

工具侧统一调用：

```rust
// diff.rs — 无模式分支
fn call(&self, args) -> ... {
    let source = super::source();
    let diff = super::repo().diff_tree_to_tree(
        source.parent_tree.as_ref(),  // None → 对空树，等价于全量 diff（自然处理初始提交）
        Some(&source.tree),
        None,
    );
}

// read / list / grep / glob — 只用 source.tree，连 parent_tree 都不碰
```

#### 三种方案对比

| 维度 | shell-out + 分支 | shell-out + 虚拟 commit | git2 + Source enum |
|------|-----------------|------------------------|-------------------|
| 工具内模式分支 | diff/stat/glob 各有 if/else | 0 | 0 |
| 需要伪造 commit 对象 | 否 | 是 | **否** |
| 全局状态 | STAGED_HASH + REVIEW_HASH | COMMIT_HASH | Repository + Source |
| 初始提交处理 | 特殊命令语法 | 特殊 diff 语法 | `parent_tree = None` 自然处理 |
| 概念简洁度 | 低 | 中 | **最高** |
| 输出解析 | 字符串切分 | 字符串切分 | 结构化对象 |
| 错误处理 | stderr + exit code | stderr + exit code | Result + typed error |

#### 迁移影响：彻底移除 `Command::new("git")`

引入 git2 后，项目中不再有任何 `Command::new("git")` 调用。所有 git 操作统一走 git2 API：

- `run_git()` 体系 → 删除，替换为 `Repository` 方法调用
- `STAGED_HASH` + `CONFIG` → `REPO` + `SOURCE` + `CONFIG`
- 字符串解析（split/trim） → 结构化对象字段访问
- `src/config.rs` 的 `Command::new("git").arg("config")` → `Repository::config()` 或 `Config::open_default()`
- `-t` 调试模式：工具签名不变，仅内部实现替换

**grep 工具的纯 Rust 实现**：当前 `git grep --perl-regexp` 不再保留，grep 改为：

```
source.tree.walk() → 逐 blob 读内容 → Rust regex crate 匹配
```

- 行为对齐目标：类 `grep -rE`，不追求 `git grep --perl-regexp` 的完整 PCRE 兼容
- `regex` crate 覆盖绝大多数正则需求（非回溯 NFA，安全无 ReDoS）
- 若 LLM 使用了 lookahead/lookbehind 等 PCRE 特性，`regex` 会返回错误，工具返回 `[Pattern not supported: ...]` 提示 LLM 简化 pattern
- 性能约束：带 path 参数时只遍历子树，不带 path 时全量遍历 + 超时兜底
- 二进制文件：每个 blob 先做 UTF-8 检测，非文本 blob 跳过

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

全局状态重构：

```rust
use std::sync::OnceLock;
use git2::Repository;

static REPO: OnceLock<Repository> = OnceLock::new();
static SOURCE: OnceLock<Source> = OnceLock::new();
static CONFIG: OnceLock<crate::config::Config> = OnceLock::new();

pub fn init_staged() -> Result<()> {
    let repo = Repository::discover(".")?;
    let source = Source::from_staged(&repo)?;
    REPO.set(repo).expect("repo already set");
    SOURCE.set(source).expect("source already set");
    Ok(())
}

pub fn init_review(hash: &str) -> Result<()> {
    let repo = Repository::discover(".")?;
    let source = Source::from_commit(&repo, hash)?;
    REPO.set(repo).expect("repo already set");
    SOURCE.set(source).expect("source already set");
    Ok(())
}

pub fn repo() -> &'static Repository {
    REPO.get().expect("repo not initialized")
}

pub fn source() -> &'static Source {
    SOURCE.get().expect("source not initialized")
}
```

工具迁移要点：

| 工具 | 当前实现 | git2 实现 |
|------|---------|----------|
| read | `git cat-file blob <hash>` + 流式读取 | `source.tree.get_path(path)?.to_object(repo)` → 读 blob |
| list | `git ls-tree --full-tree <tree>` | `source.tree.walk(TreeWalkMode::PreOrder, callback)` |
| grep | `git grep --perl-regexp` | `source.tree.walk()` + 逐 blob `regex` 匹配 |
| glob | `git ls-files --cached --glob-pathspecs` | `source.tree.walk()` + Rust glob pattern 过滤 |
| diff | `git diff --cached` | `repo.diff_tree_to_tree(parent_tree, tree, None)` |
| stat | `git diff --cached --numstat` | 从 `Diff` 对象的 delta/stats 提取 |
| log | `git log --format` | `repo.revwalk()` + commit message 提取 |
| cache | 纯文件系统 | 不变 |
| subagent | 组合 read/list/grep/glob | 不变 |

配置迁移：

| 当前实现 | git2 实现 |
|---------|----------|
| `Command::new("git").arg("config").arg("--get")` | `repo.config()?.get_entry("vibe.auth-key")` 或 `Config::open_default()` |

### 工具迁移难度评估

迁移原则：行为对齐目标为常见 bash 工具（`grep -rE`、`find -name`），非 git 子命令语义。

| 难度 | 工具 | 要点 |
|------|------|------|
| 中等 | log | `repo.revwalk()` + 格式化输出，体力活无难点 |
| 低 | grep | `tree.walk()` + `regex`，PCRE 不再需要兼容，带 path 时只遍历子树，无 path 时全量 + 超时 |
| 低 | glob | `tree.walk()` + `glob::Pattern::matches_path()`，标准 glob 语义，非 git pathspec |
| 低 | read | blob 读取 + 复用现有 `is_utf8_text()` 二进制检测逻辑 |
| 低 | diff / stat | 结构化 `Diff` 对象，比字符串解析更简单 |
| 低 | list | `tree.walk()` 天然等价 |
| 无 | cache / subagent | 不涉及 git |

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
Mode::Commit(editmsg_path) => {
    tool::init_staged()?;
    tool::ensure_cache_dir();
    // ... 现有 commit 逻辑
}

Mode::Review(hash) => {
    tool::init_review(&hash)?;
    tool::ensure_cache_dir();
    let output = llm::review(&hash).await?;
    println!("{}", output);
}
```

### 6. 实施顺序

1. 添加 `git2` 依赖到 `Cargo.toml`
2. 重构 `src/tool/mod.rs`：引入 `Repository` + `Source`，替换 `STAGED_HASH` + `run_git()`
3. 逐个迁移工具：diff → stat → read → list → glob → grep → log
4. 迁移 `src/config.rs`：`Command::new("git").arg("config")` → git2 `Config` API
5. 删除 `run_git()` 和所有 `Command::new("git")` 调用
6. 更新 `AGENTS.md`，移除 "No git libraries" 规则
7. 确保现有 bats 测试全部通过
8. 实现新增：`Mode::Review`、`review.txt`、`llm::review()`
9. 新增 `tests/review.bats`

### 7. 测试策略

- **迁移验证**：现有 bats 测试全部通过，确保 git2 迁移无行为回归
- **工具层**：`-t diff` / `-t stat` / `-t read` 等调试模式验证 git2 实现
- **集成层**：新增 `tests/review.bats`
  - 正常 commit 的审阅输出
  - 不存在的 hash 报错
  - merge commit（默认取第一父）
  - 初始提交（无父 commit）

### 8. 已决事项

| 事项 | 决定 | 理由 |
|------|------|------|
| git 访问方式 | git2 替代 shell-out，彻底移除 `Command::new("git")` | Source enum 统一数据源，工具零分支，结构化 API |
| 模式切换实现 | Source enum（tree + parent_tree） | 不需要虚拟 commit，无 cleanup，初始提交自然处理 |
| merge commit | 默认取第一父 | 与 `git show` 行为一致，用户可通过 `^2` 手动指定 |
| review.txt 与 commit.txt | 独立 | 步骤 7 差异大，共享模板增加维护耦合 |
| grep 工具 | `source.tree.walk()` + Rust `regex` crate | 行为对齐 `grep -rE`，不追求 PCRE 兼容；全量遍历加超时兜底 |
| config 读取 | git2 `Config` API | 不保留 shell-out，与工具层统一 |
| 工具行为标准 | 对齐常见 bash 工具，非 git 子命令语义 | grep 对齐 `grep -rE`，glob 对齐 `find -name`，降低实现复杂度 |
| Review 模式的 progress 工具 | 暂不需要 | 与 commit agent 同步启用 |
