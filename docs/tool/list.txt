List directory contents from the staged repository snapshot. If the path
does not exist or is a file, an error is returned.

Usage:
- The `path` parameter should be a repository-absolute path.
- Paths start from `/`, where `/` refers to the git repository root rather
  than the local filesystem root. For example, `/src` refers to the
  project's `src/` directory in the repository root.
- Returns directory entries sorted alphabetically, one per line, with a
  trailing `/` for subdirectories.
- Directory output is capped at 50 KB per call. When capped, use the glob
  tool with a more specific pattern to narrow the search.
- Use this tool when you need to explore the directory structure.
- To read file contents, use the read tool instead.
