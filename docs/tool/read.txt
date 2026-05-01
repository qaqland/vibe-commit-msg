Read a file from the staged repository snapshot. If the path does not
exist or is not a valid UTF-8 text file, an error is returned.

Usage:
- The `path` parameter should be a repository-absolute path.
- Paths start from `/`, where `/` refers to the git repository root rather
  than the local filesystem root. For example, `/README.md` refers to the
  project's README.md in the repository root.
- By default, this tool returns up to 2000 lines from the requested offset.
- The `offset` parameter is 1-indexed for file lines.
- The `limit` parameter must be at least 1.
- To read later sections, call this tool again with a larger offset.
- Use the grep tool to find specific content in large files or files with
  long lines.
- If you are unsure of the correct file path, use the glob tool to look up
  filenames by glob pattern.
- Contents are returned with each line prefixed by its line number as
  `<line>: <content>`. For example, if a file has contents `foo\n`, you
  will receive `1: foo\n`.
- Any line longer than 2000 characters is truncated.
- File output is capped at 50 KB per call. When capped, use the returned
  `offset` hint to continue.
- Call this tool in parallel when you know there are multiple files to read.
- Avoid tiny repeated slices (30 line chunks). If you need more context,
  read a larger window.
- This tool only supports reading valid UTF-8 text files.
- To list directory contents, use the list tool instead.
