# vibe-commit-msg

LLM-powered commit message generator from staged changes.

## Configuration

Set via git config (see `gitconfig.example`):

```
git config vibe.auth-key <key>
git config vibe.base-url <url>    # default: https://api.openai.com/v1
git config vibe.model-id <model>  # default: gpt-5.2
```

## Usage

Install as a `prepare-commit-msg` hook by symlinking:

```
ln -s "$(which vibe-commit-msg)" .git/hooks/prepare-commit-msg
```

## How It Works

A multi-agent pipeline analyzes staged changes using up to four specialized
LLM calls. **Summary** and **Style** agents (invoked via `-s` or when caches
are stale) explore the repository to build project context and infer commit
style conventions. **Commit** agent examines the staged diff to produce a
structured change summary, which **Message** agent formats into the final
commit message.

