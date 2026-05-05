# Rules

Do not use git2-rs, gitoxide, or any other git library. The user's system
always has `git` installed; invoke it directly via `Command::new("git")`
instead of pulling in extra dependencies.

Configuration must be read from `git config`, not from standalone config
files. Keep config in git's ecosystem; see `src/config.rs` for the existing
pattern.

Do not add Co-authored-by or other author metadata fields to commit messages.
Commit authorship is the user's responsibility; the AI should not inject it.

Do not write unit tests. The project uses bats for integration testing;
all test coverage should go through bats-based integration tests.

Use only OpenAI-compatible APIs, and never use streaming. All LLM calls go
through `rig::providers::openai` with a configurable `base_url`, and must use
the non-streaming `prompt()` method.

