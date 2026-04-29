Find files by glob pattern against the staged repository snapshot. If no
files match, `[No files found]` is returned.

Usage:
- The `pattern` parameter is required and supports glob patterns (e.g.
  `**/*.rs`, `src/**/*.ts`).
- The search always starts from the repository root `/`.
- Returns matching file paths sorted alphabetically, one per line. Paths
  start from `/`.
- Results are limited to 100 files. When truncated, use a more specific
  pattern to narrow the search.
- Use this tool when you need to find files by name patterns.
