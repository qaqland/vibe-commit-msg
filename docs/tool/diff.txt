Show staged changes as a unified diff. If `path` is provided, shows diff
for that specific file only; otherwise shows all staged changes.

Returns the full diff text, or `[No changes found]` if there are no staged
changes. Output is capped at 50 KB at line boundaries. When the diff is
truncated, use `stat` to identify files, then call `diff` with a specific
path.
