# AGENTS.md

## Project Constraints (from RULES.md)
- **No git libraries** — shell out to `Command::new("git")` only. No git2-rs, gitoxide, etc.
- **Config via `git config` only** — no standalone config files; see `src/config.rs`.
- **No author metadata in commit messages** — no Co-authored-by or similar. Authorship is the user's responsibility.
- **No unit tests** — all testing is bats-based integration tests in `tests/`.
- **OpenAI-compatible API only, never stream** — use `rig::providers::openai` with non-streaming `prompt()`.

## Repo Shape
- Single-crate Rust CLI (`edition = "2024"` — requires Rust 1.85+). Binary `src/main.rs`; lib modules in `src/lib.rs`.
- Core logic: `src/llm.rs`, `src/config.rs`, `src/tool/` (9 tools + shared helpers in `src/tool/mod.rs`).
- Tool descriptions live in `docs/tool/*.txt`, compiled in via the `tool_description!` macro in `src/tool/mod.rs`.
- LLM prompt templates live in `docs/prompt/*.txt`, compiled in via the `prompt_description!` macro in `src/llm.rs`.
- Tool structs re-exported from `src/tool/mod.rs`: `pub use <module>::<Name>`. Use as `crate::tool::{Read, List, …}`.
- Uses `rig-core` for LLM agent/tool orchestration (OpenAI-compatible API).

## Runtime Model
- `staged_hash()` in `src/tool/mod.rs` does `git rev-parse --show-toplevel` → `cd` → `git write-tree`, stored in a `OnceLock`. Called once at startup (all modes except Help).
- `set_config()` in `src/tool/mod.rs` stores `Config` in a `OnceLock`. Called once at startup after `Config::load()`. The **subagent** tool reads it to build its own LLM client.
- Tools that read file content/listings from the frozen tree hash: **read**, **list**, **grep** (via `git ls-tree`/`git cat-file`/`git grep --null <tree>`).
- Tools that read from the live index (not the tree hash): **diff** (`git diff --cached`), **stat** (`git diff --cached --numstat`), **glob** (`git ls-files --cached`). They see what `git add` staged; in practice identical to the tree since the index doesn't change during execution.
- **log** operates on commit history (HEAD), unrelated to the staged snapshot.
- **subagent** launches a read-only explore agent (tools: read, list, grep, glob) with a specific task and thoroughness level. Used by summarize and commit orchestrators for deeper investigation
- All tool paths are **repository-absolute** strings starting with `/` (e.g. `/src/main.rs`). Enforced by read/list/log. Paths are passed as-is to `git ls-tree`/`git cat-file`.
- **Must be run inside a git repo.**

## Config
- LLM config read from **git config** (not env vars), by `src/config.rs`:
  - `vibe.auth-key` (required — CLI errors without it)
  - `vibe.base-url` (optional; defaults to `https://api.openai.com/v1`)
  - `vibe.model-id` (optional; defaults to `gpt-5.2`)
- See `gitconfig.example` for setup. Designed as a `prepare-commit-msg` hook (see README).

## Commit Pipeline
Commit mode is a two-step LLM process, not a single call:
1. `llm::commit()` — agent with 6 tools (Cache, Diff, Stat, Read, Grep, Glob) produces a structured change summary
2. `llm::message()` — plain completion (no tools) combines the summary with the style cache to produce the final message

Before step 1, caches are auto-refreshed if stale (>7 days, see `STALENESS_SECS` in `cache.rs`).

## Retry & Progress
- Every `agent.prompt()` goes through `prompt_with_retry()` in `src/llm.rs` (also used by the subagent tool): up to 4 attempts, exponential backoff 2s → 4s → 8s (capped at 30s).
- Retryable: connection errors, 408/429/5xx, malformed responses (`JsonError`/`ResponseError`). Not retried: other 4xx (bad request/auth), tool errors, max-turns. Note: rig surfaces HTTP statuses as `http_client::Error::InvalidStatusCode*` inside `CompletionError::HttpError`.
- A retry restarts the whole agent loop from the first turn; tool calls re-run (they are read-only or idempotent cache writes).

## Mode → LLM Function → Tool Assignment
| Mode | Flag | LLM calls | Tools available to agent |
|------|------|-----------|--------------------------|
| Commit | (none) | `commit()` → `message()` | Cache, Diff, Stat, Read, Grep, Glob, Subagent (commit); none (message) |
| Summary | `-s` | `summarize()` + `style()` | List, Glob, Grep, Read, Diff, Stat, Log, Cache, Subagent (summarize); Glob, Grep, Read, Log, Cache (style) |
| Debug tool | `-t` | none (local `run_tool`) | all 9 (not LLM, just dispatched locally) |
| Translate | `-e` | `translator()` | none (plain completion) |

## Cache System
- Cache directory: `.git/vibe/` inside the repo (not `~/.cache/` — the README is aspirational).
- Files are `.md` with YAML front matter (`---` … `---`). `updated_at` is auto-injected on write.
- Staleness threshold: 7 days. Commit mode auto-refreshes when either `style.md` or `project.md` is stale.
- Filenames: must be valid identifiers (no `..`, `/`; only `[a-zA-Z0-9_.-]`). Capped at 8 KB per file.

## Tool Constraints (enforced by `paginate_output` in `src/tool/mod.rs`)
- **read**: default 2000 lines, capped at 50 KB. Use `offset`/`limit` to paginate.
- **list**: capped at 200 entries / 50 KB. No offset/limit — use `glob` for narrower lookup.
- **grep**: uses `--perl-regexp` (PCRE) with `--null` for safe NUL-separated parsing. Capped at 100 matches / 50 KB.
- **diff** / **stat**: capped at 50 KB (no line limit). Stat capped at 200 entries.
- **log**: default 20 commits, max 100. Capped at 50 KB.
- Truncation notices appended (e.g. `[Showing 1-100 of 523...]`).

## Verification
```
cargo check                       # fast compile check
cargo fmt --check                 # formatting
make test                         # build release + run all bats tests
VIBE_BIN=./target/release/vibe-commit-msg bats tests/read.bats  # single test file
cargo build --release             # build only (tests require release binary)
cargo run --release -- -t <tool> '<json>'  # debug a single tool
```
Tests require `bats-core` with `bats-support` and `bats-assert` libraries. Each test creates a temp git repo (`setup()` in `tests/test_helper.bash`).

## Commands
| Flag | Description |
|------|-------------|
| (none) | Generate commit message from staged changes |
| `-s` | Refresh project + style caches |
| `-e "<text>"` | Translate Chinese/mixed text to concise English commit text |
| `-t <name> '<json>'` | Run a single tool for debugging |
| `-h` | Show help |
| `VIBE_DEBUG=1` | Enable error chain trace in any mode |

## Known Issues
(none currently)
