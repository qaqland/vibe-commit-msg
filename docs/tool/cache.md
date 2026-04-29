Manage project memory cache files. The cache directory persists across
runs (`.git/vibe/`). Each file is a standalone
markdown document with YAML front matter.

Three modes depending on arguments:
- No `path`, no `content`: List all cached `.md` files with their front
  matter fields (name, description, etc.).
- `path` without `content`: Read the file and return its full content.
  Returns `[File not found]` if absent.
- `path` with `content`: Validate and write (overwrite) the file. Files
  are always written in full, not patched. Content MUST have valid YAML
  front matter at the top (`---` on first line, key: value pairs, closing
  `---`). Maximum file size is 8 KB. Writes failing YAML validation or
  exceeding the size limit are rejected with an error. The `updated_at`
  field in front matter is automatically set to the current unix timestamp
  on every write — use `0` as a placeholder value.

Files should be kept concise so the LLM can rewrite them in one shot.
Use `cache` without arguments to see what files exist, then read the
ones you need to review, and write updated versions.
