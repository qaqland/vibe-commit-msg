# AGENTS.md

## Repo Shape
- Single-crate Rust CLI (`edition = "2024"` — requires Rust 1.85+). Binary `src/main.rs`; lib modules in `src/lib.rs`.
- Core logic: `src/llm.rs`, `src/config.rs`, `src/tool/` (8 tools + shared helpers in `src/tool/mod.rs`).
- Tool descriptions live in `docs/tool/*.txt`, compiled in via the `tool_description!` macro in `src/tool/mod.rs`.
- LLM prompt templates live in `docs/prompt/*.txt`, compiled in via the `prompt_description!` macro in `src/llm.rs`.
- Tool structs re-exported from `src/tool/mod.rs`: `pub use <module>::<Name>`. Use as `crate::tool::{Read, List, …}`.
- Uses `rig-core` for LLM agent/tool orchestration (OpenAI-compatible API).

## Runtime Model
- `staged_hash()` in `src/tool/mod.rs` does `git rev-parse --show-toplevel` → `cd` → `git write-tree`, stored in a `OnceLock`. Called once at startup (all modes except Help).
- Tools that read file content/listings from the frozen tree hash: **read**, **list**, **grep** (via `git ls-tree`/`git cat-file`/`git grep <tree>`).
- Tools that read from the live index (not the tree hash): **diff** (`git diff --cached`), **stat** (`git diff --cached --numstat`), **glob** (`git ls-files --cached`). They see what `git add` staged; in practice identical to the tree since the index doesn't change during execution.
- **log** operates on commit history (HEAD), unrelated to the staged snapshot.
- All tool paths are **repository-absolute** strings starting with `/` (e.g. `/src/main.rs`). Enforced by read/list/diff/log. Paths are passed as-is to `git ls-tree`/`git cat-file`.
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

## Mode → LLM Function → Tool Assignment
| Mode | Flag | LLM calls | Tools available to agent |
|------|------|-----------|--------------------------|
| Commit | (none) | `commit()` → `message()` | Cache, Diff, Stat, Read, Grep, Glob (commit); none (message) |
| Summary | `-s` | `summarize()` + `style()` | List, Glob, Grep, Read, Diff, Stat, Log, Cache (summarize); Glob, Grep, Read, Log, Cache (style) |
| Debug tool | `-t` | none (local `run_tool`) | all 8 (not LLM, just dispatched locally) |
| Translate | `-e` | `translator()` | none (plain completion) |

## Cache System
- Cache directory: `.git/vibe/` inside the repo (not `~/.cache/` — the README is aspirational).
- Files are `.md` with YAML front matter (`---` … `---`). `updated_at` is auto-injected on write.
- Staleness threshold: 7 days. Commit mode auto-refreshes when either `style.md` or `project.md` is stale.
- Filenames: must be valid identifiers (no `..`, `/`; only `[a-zA-Z0-9_.-]`). Capped at 8 KB per file.

## Tool Constraints (enforced by `paginate_output` in `src/tool/mod.rs`)
- **read**: default 2000 lines, capped at 50 KB. Use `offset`/`limit` to paginate.
- **list**: capped at 200 entries / 50 KB. No offset/limit — use `glob` for narrower lookup.
- **grep** / **glob**: capped at 100 matches/files.
- **diff** / **stat**: capped at 50 KB (no line limit). Stat capped at 200 entries.
- **log**: default 20 commits, max 100. Capped at 50 KB.
- Truncation notices appended (e.g. `[Showing 1-100 of 523...]`).

## Verification
```
cargo check          # fast compile check (no tests exist)
cargo fmt --check    # formatting
cargo run -- -t <tool> '<json>'   # debug a single tool
```

## Commands
| Flag | Description |
|------|-------------|
| (none) | Generate commit message from staged changes |
| `-s` | Refresh project + style caches |
| `-e "<text>"` | Translate Chinese/mixed text to concise English commit text |
| `-t <name> '<json>'` | Run a single tool for debugging |
| `-h` | Show help |
| `VIBE_DEBUG=1` | Enable error chain trace in any mode |

## Unimplemented / Known Issues
- Line truncation at 2000 chars — documented in tool descriptions but not enforced in code.
- `grep` uses `--basic-regexp` (BRE) but description claims "full regex syntax".
- `parse_grep_line` in `grep.rs` uses `rsplit_once(':')` which breaks on file paths containing `:`.
- `log` uses `--author` with default `"."` (matches all authors) — the regex filter is always applied.
